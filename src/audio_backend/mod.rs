use std::time::Duration;

use crossbeam::{Receiver, Sender};
use rodio::{Sink, Source};

use done_access::DoneAccess;
use periodic_access::PeriodicAccess;
use start_access::StartAccess;

use crate::audio_backend::audio_buffer::{AudioBuffer, OpenError};
use crate::song::{Song, SongID};

mod done_access;
mod start_access;
mod periodic_access;
mod audio_buffer;
mod arc_samples_buffer;

const UPDATE_DURATION: Duration = Duration::from_millis(100);

pub struct AudioBackend {
	sink: rodio::Sink,
	_stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
    info_sender: Sender<AudioInfo>, // sender for info to musicus
	update_sender: Sender<AudioBackendCommand>, // internal updates for source state
	current_song: Option<CurrentSongState>, //
	audio_buffer: AudioBuffer,
	volume: f32,
}

struct CurrentSongState {
	song: Song,
	total_duration: Duration,
	play_duration: Duration,
	start_duration: Duration,
}

impl CurrentSongState {
	fn get_real_play_duration(&self) -> Duration {
		self.play_duration + self.start_duration
	}

	fn set_real_play_duration(&mut self, duration: Duration) {
		self.play_duration = duration.checked_sub(self.start_duration).unwrap_or(Duration::new(0, 0));
	}
}

pub enum AudioCommand {
	Play(Song),
	Queue(Song),
	Load(Song),
	Pause,
    Unpause,
	Seek(SeekCommand),
    SetVolume(f32),
}

#[derive(Copy, Clone)]
pub struct SeekCommand {
	pub duration: Duration,
	pub direction: SeekDirection,
}

impl SeekCommand {
	fn as_secs(&self) -> f64 {
		match self.direction {
			SeekDirection::Forward => self.duration.as_secs_f64(),
			SeekDirection::Backward => -self.duration.as_secs_f64(),
		}
	}

	fn from_secs(secs: f64) -> SeekCommand {
		SeekCommand {
			duration: Duration::from_secs_f64(secs.abs()),
			direction: if secs > 0.0 { SeekDirection::Forward } else { SeekDirection::Backward },
		}
	}

	fn join(a: SeekCommand, b: SeekCommand) -> SeekCommand {
		SeekCommand::from_secs(a.as_secs() + b.as_secs())
	}
}

#[derive(Copy, Clone)]
pub enum SeekDirection {
	Forward,
	Backward,
}

#[derive(Debug)]
pub enum AudioInfo {
	Playing(Song, Duration, Duration), // playing song, play duration, total duration
	Queued(Song),
	SongStarts(Song, Duration, Duration),
	FailedOpen(Song, OpenError),
	SongEnded(Song),
	SongDuration(SongID, Duration),
}

#[derive(Debug)]
pub struct PlayingUpdate {
	song: Song,
	duration_played: Duration,
}

#[derive(Debug)]
pub enum AudioUpdate {
	Playing(PlayingUpdate),
	SongStarts(Song, Duration, Duration), // song played, total duration, start duration
	SongEnded(Song),
}

pub enum AudioBackendCommand {
	Command(AudioCommand),
	Update(AudioUpdate),
}

impl AudioBackendCommand {
	pub fn simplify(vec: Vec<AudioBackendCommand>) -> Vec<AudioBackendCommand> {
		let mut result = Vec::new();
		let mut last_play_command = None;
		let mut last_playing_update = None;
		let mut seek_command: Option<SeekCommand> = None;
		let mut last_set_volume: Option<f32> = None;

		for command_or_update in vec.into_iter() {
			match command_or_update {
				AudioBackendCommand::Command(command) => {
					match command {
						AudioCommand::Play(play) => {
							last_play_command = Some(play);
						}
						AudioCommand::Seek(new_seek) => {
							seek_command = seek_command.map_or(Some(new_seek), |old_seek| Some(SeekCommand::join(old_seek, new_seek)))
						}
						AudioCommand::SetVolume(v) => {
							last_set_volume = Some(v);
						}
						command => {
							result.push(AudioBackendCommand::Command(command));
						}
					}
				}
				AudioBackendCommand::Update(update) => {
					match update {
						AudioUpdate::Playing(playing) => last_playing_update = Some(playing),
						update => result.push(AudioBackendCommand::Update(update)),
					}
				}
			}
		}
		if let Some(play_command) = last_play_command {
			result.push(AudioBackendCommand::Command(AudioCommand::Play(play_command)));
		}
		if let Some(playing_update) = last_playing_update {
			result.push(AudioBackendCommand::Update(AudioUpdate::Playing(playing_update)))
		}
		if let Some(seek_command) = seek_command {
			result.push(AudioBackendCommand::Command(AudioCommand::Seek(seek_command)));
		}
		if let Some(v) = last_set_volume {
			result.push(AudioBackendCommand::Command(AudioCommand::SetVolume(v)));
		}
		result
	}
}

impl AudioBackend {
	pub fn new(info_sender: Sender<AudioInfo>, audio_backend_sender: Sender<AudioBackendCommand>, volume: f32) -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			_stream: stream,
			stream_handle,
			info_sender,
			update_sender: audio_backend_sender,
			current_song: None,
			audio_buffer: AudioBuffer::new(),
			volume,
		}
	}

	pub fn run(&mut self, audio_backend_receiver: Receiver<AudioBackendCommand>) {
		loop {
			let command = match audio_backend_receiver.recv() {
				Ok(command) => command,
				Err(_) => break,
			};

			let mut commands = vec![command];
			commands.extend(audio_backend_receiver.try_iter());
			let commands = AudioBackendCommand::simplify(commands);
			for command in commands.into_iter() {
				match command {
					AudioBackendCommand::Command(command) => self.handle_command(command),
					AudioBackendCommand::Update(update) => self.handle_update(update),
				}
			}
		}
	}

	fn handle_command(&mut self, command: AudioCommand) {
		match command {
			AudioCommand::Play(song) => Self::play(&mut self.sink, &self.stream_handle, &self.update_sender, &self.info_sender, &song, None, &self.audio_buffer, self.volume),
			AudioCommand::Queue(song) => Self::queue(&mut self.sink, &self.update_sender, &self.info_sender, &song, None, &self.audio_buffer),
			AudioCommand::Load(song) => self.audio_buffer.load(song.get_path().to_path_buf()),
			AudioCommand::Pause => self.pause(),
			AudioCommand::Unpause => self.unpause(),
			AudioCommand::Seek(seek_command) => self.seek(seek_command),
			AudioCommand::SetVolume(volume) => self.set_volume(volume),
		}
	}

	fn set_volume(&mut self, volume: f32) {
		self.sink.set_volume(volume);
		self.volume = volume;
	}

	fn handle_update(&mut self, update: AudioUpdate) {
		match update {
			AudioUpdate::Playing(playing_update) => {
				if let Some(current_song) = &mut self.current_song {
					assert_eq!(current_song.song.get_path(), playing_update.song.get_path());
					current_song.set_real_play_duration(playing_update.duration_played);
					self.info_sender.send(AudioInfo::Playing(playing_update.song.clone(), current_song.get_real_play_duration(), current_song.total_duration)).unwrap();
				}
			}
			AudioUpdate::SongEnded(path) => {
				self.info_sender.send(AudioInfo::SongEnded(path)).unwrap();
				self.current_song = None;
			}
			AudioUpdate::SongStarts(song, total_duration, start_duration) => {
				self.info_sender.send(AudioInfo::SongStarts(song.clone(), total_duration, start_duration)).unwrap();
				self.current_song = Some(CurrentSongState {
					song,
					total_duration,
					play_duration: Duration::new(0, 0),
					start_duration,
				});
			}
		}
	}

	fn seek(&mut self, seek_command: SeekCommand) {
		if let Some(current_song) = &mut self.current_song {
			current_song.start_duration = match seek_command.direction {
				SeekDirection::Forward => {
					(current_song.get_real_play_duration() + seek_command.duration).min(current_song.total_duration)
				}
				SeekDirection::Backward => {
					current_song.get_real_play_duration().checked_sub(seek_command.duration).unwrap_or(Duration::new(0, 0))
				}
			};
			current_song.play_duration = Duration::new(0, 0);
			Self::play(&mut self.sink, &self.stream_handle, &self.update_sender, &self.info_sender, &current_song.song, Some(current_song.get_real_play_duration()), &self.audio_buffer, self.volume);
		}
	}

	fn play(
		sink: &mut Sink,
		stream_handle: &rodio::OutputStreamHandle,
		update_sender: &Sender<AudioBackendCommand>,
		info_sender: &Sender<AudioInfo>,
		song: &Song,
		skip: Option<Duration>,
		audio_buffer: &AudioBuffer,
        volume: f32,
	) {
		if !sink.empty() {
			sink.stop();
			*sink = rodio::Sink::try_new(stream_handle).unwrap();
		}
		sink.set_volume(volume);
		Self::queue(sink, update_sender, info_sender, &song, skip, audio_buffer);
		sink.play();
	}

	fn queue(
		sink: &mut Sink,
		orig_update_sender: &Sender<AudioBackendCommand>,
		info_sender: &Sender<AudioInfo>,
		song: &Song,
		skip: Option<Duration>,
		audio_buffer: &AudioBuffer,
	) {
		match audio_buffer.get(song.get_path()) {
			Ok(song_buffer) => {
				if let Some(total_duration) = song_buffer.total_duration() {
					// send total duration info
					if song.get_total_duration().is_none() {
						info_sender.send(AudioInfo::SongDuration(song.get_id(), total_duration)).unwrap();
					}

					// add start info
					let update_sender = orig_update_sender.clone();
					let song_copy = song.clone();
					let start_access_source = StartAccess::new(
						song_buffer,
						move || update_sender.send(
							AudioBackendCommand::Update(AudioUpdate::SongStarts(
								song_copy.clone(), total_duration, skip.unwrap_or(Duration::new(0, 0))
							))
						).unwrap(),
					);

					// add playing info
					let update_sender = orig_update_sender.clone();
					let song_copy = song.clone();
					let periodic_access_source = PeriodicAccess::new(
						start_access_source,
						move |_source, duration_played| {
							update_sender.send(
								AudioBackendCommand::Update(AudioUpdate::Playing(
									PlayingUpdate {
										song: song_copy.clone(),
										duration_played,
									}
								)
								)).unwrap();
						},
						UPDATE_DURATION
					);

					// add done info
					let update_sender = orig_update_sender.clone();
					let song_copy = song.clone();
					let done_access_source = DoneAccess::new(
						periodic_access_source,
						move |_| update_sender.send(
							AudioBackendCommand::Update(AudioUpdate::SongEnded(song_copy.clone()))
						).unwrap(),
					);

					if let Some(duration) = skip {
						let source = done_access_source.skip_duration(duration);
						sink.append(source);
					} else {
						sink.append(done_access_source);
					}

					info_sender.send(AudioInfo::Queued(song.clone())).unwrap();
				}
			},
			Err(e) => {
				info_sender.send(AudioInfo::FailedOpen(song.clone(), e)).unwrap();
			}
		}
	}

	fn pause(&mut self) {
		self.sink.pause();
	}

	fn unpause(&mut self) {
		self.sink.play();
	}
}
