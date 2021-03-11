use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossbeam::{Receiver, Sender};
use rodio::{Sink, Source};

use done_access::DoneAccess;
use periodic_access::PeriodicAccess;
use start_access::StartAccess;

use crate::musicus::log;

mod done_access;
mod start_access;
mod periodic_access;

const UPDATE_DURATION: Duration = Duration::from_millis(100);
const SONG_ENDS_SOON_OFFSET: Duration = Duration::from_millis(2000);

pub struct AudioBackend {
	sink: rodio::Sink,
	_stream: rodio::OutputStream,
	stream_handle: rodio::OutputStreamHandle,
    info_sender: Sender<AudioInfo>, // sender for info to musicus
	update_sender: Sender<AudioBackendCommand>, // internal updates for source state
	current_song: Option<CurrentSongState>, //
}

struct CurrentSongState {
	path: PathBuf,
	total_duration: Duration,
	current_duration: Duration,
	start_duration: Duration,
	sent_song_ends_soon: bool,
}

impl CurrentSongState {
	fn get_real_current_duration(&self) -> Duration {
		self.current_duration + self.start_duration
	}

	fn set_real_current_duration(&mut self, duration: Duration) {
		self.current_duration = duration.checked_sub(self.start_duration).unwrap_or(Duration::new(0, 0));
	}
}

pub enum AudioCommand {
	Play(PathBuf),
	Queue(PathBuf),
	Pause,
    Unpause,
	Seek(SeekCommand),
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
	Playing(PathBuf, Duration), // current song, current duration
	Queued(PathBuf),
	SongStarts(PathBuf, Duration, Duration),
	SongEndsSoon(PathBuf, Duration),
	FailedOpen(PathBuf),
	SongEnded(PathBuf),
}

#[derive(Debug)]
pub struct PlayingUpdate {
	song_path: PathBuf,
	duration_left: Duration,
}

#[derive(Debug)]
pub enum AudioUpdate {
	Playing(PlayingUpdate), // current song, left duration
	SongStarts(PathBuf, Duration, Duration), // current song, total duration, start duration
	SongEnded(PathBuf),
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
		result
	}
}

impl AudioBackend {
	pub fn new(info_sender: Sender<AudioInfo>, audio_backend_sender: Sender<AudioBackendCommand>) -> AudioBackend {
		let (stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
		AudioBackend {
			sink: rodio::Sink::try_new(&stream_handle).unwrap(),
			_stream: stream,
			stream_handle,
			info_sender,
			update_sender: audio_backend_sender,
			current_song: None,
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
			AudioCommand::Play(path) => Self::play(&mut self.sink, &self.stream_handle, &self.update_sender, &self.info_sender, &path, None),
			AudioCommand::Queue(path) => Self::queue(&mut self.sink, &self.update_sender, &self.info_sender, &path, None),
			AudioCommand::Pause => self.pause(),
			AudioCommand::Unpause => self.unpause(),
			AudioCommand::Seek(seek_command) => self.seek(seek_command),
		}
	}

	fn handle_update(&mut self, update: AudioUpdate) {
		match update {
			AudioUpdate::Playing(playing_update) => {
				if let Some(current_song) = &mut self.current_song {
					assert_eq!(current_song.path, playing_update.song_path);
					if !current_song.sent_song_ends_soon && playing_update.duration_left <= SONG_ENDS_SOON_OFFSET {
						self.info_sender.send(AudioInfo::SongEndsSoon(playing_update.song_path.clone(), playing_update.duration_left)).unwrap();
						current_song.sent_song_ends_soon = true;
					}
					current_song.set_real_current_duration(current_song.total_duration - playing_update.duration_left);
					// log(&format!("playing update: {:?}\n", current_song.get_real_current_duration()));
					self.info_sender.send(AudioInfo::Playing(playing_update.song_path.clone(), current_song.get_real_current_duration())).unwrap();
				} else {
					log(&format!("ERROR: current song is None, but got Playing update\n"));
				}
			}
			AudioUpdate::SongEnded(path) => {
				self.info_sender.send(AudioInfo::SongEnded(path)).unwrap();
				self.current_song = None;
			}
			AudioUpdate::SongStarts(path, total_duration, start_duration) => {
				self.info_sender.send(AudioInfo::SongStarts(path.clone(), total_duration, start_duration)).unwrap();
				self.current_song = Some(CurrentSongState {
					path: path.to_path_buf(),
					total_duration,
					current_duration: Duration::new(0, 0),
					start_duration,
					sent_song_ends_soon: false,
				});
				// log(&format!("start update: {:?}\n", start_duration));
			}
		}
	}

	fn seek(&mut self, seek_command: SeekCommand) {
		if let Some(current_song) = &mut self.current_song {
			current_song.start_duration = match seek_command.direction {
				SeekDirection::Forward => {
					(current_song.get_real_current_duration() + seek_command.duration).min(current_song.total_duration)
				}
				SeekDirection::Backward => {
					current_song.get_real_current_duration().checked_sub(seek_command.duration).unwrap_or(Duration::new(0, 0))
				}
			};
			current_song.current_duration = Duration::new(0, 0);
			// log(&format!("seek: {:?}\n", current_song.get_real_current_duration()));
			Self::play(&mut self.sink, &self.stream_handle, &self.update_sender, &self.info_sender, &current_song.path, Some(current_song.get_real_current_duration()));
		}
	}

	fn play(
		sink: &mut Sink,
		stream_handle: &rodio::OutputStreamHandle,
		update_sender: &Sender<AudioBackendCommand>,
		info_sender: &Sender<AudioInfo>,
		path: &Path,
		skip: Option<Duration>,
	) {
		if !sink.empty() {
			sink.stop();
			*sink = rodio::Sink::try_new(stream_handle).unwrap();
		}
		Self::queue(sink, update_sender, info_sender, &path, skip);
		sink.play();
	}

	fn queue(
		sink: &mut Sink,
		orig_update_sender: &Sender<AudioBackendCommand>,
		info_sender: &Sender<AudioInfo>,
		path: &Path,
		skip: Option<Duration>,
	) {
		match File::open(path) {
			Ok(file) => {
				if let Ok(decoder) = rodio::Decoder::new(BufReader::new(file)) {
					if let Some(total_duration) = decoder.total_duration() {
						// add start info
						let update_sender = orig_update_sender.clone();
						let path_buf = path.to_path_buf();
						let start_access_source = StartAccess::new(
							decoder,
							move || update_sender.send(
								AudioBackendCommand::Update(AudioUpdate::SongStarts(
									path_buf.clone(), total_duration, skip.unwrap_or(Duration::new(0, 0))
								))
							).unwrap(),
						);

						// add playing info
						let update_sender = orig_update_sender.clone();
						let path_buf = path.to_path_buf();
						let periodic_access_source = PeriodicAccess::new(
							start_access_source,
							move |s| update_sender.send(
								AudioBackendCommand::Update(AudioUpdate::Playing(
									PlayingUpdate {
										song_path: path_buf.clone(),
										duration_left: s.total_duration().unwrap_or(Duration::new(0, 0)),
									}
								)
								)).unwrap(),
							UPDATE_DURATION
						);

						// add done info
						let update_sender = orig_update_sender.clone();
						let path_buf = path.to_path_buf();
						let done_access_source = DoneAccess::new(
							periodic_access_source,
							move |_| update_sender.send(
								AudioBackendCommand::Update(AudioUpdate::SongEnded(path_buf.clone()))
							).unwrap(),
						);

						if let Some(duration) = skip {
							let source = done_access_source.skip_duration(duration);
							sink.append(source);
						} else {
							sink.append(done_access_source);
						}

						info_sender.send(AudioInfo::Queued(path.to_path_buf())).unwrap();
					} else {
						info_sender.send(AudioInfo::FailedOpen(path.to_path_buf())).unwrap();
					}
				} else {
					info_sender.send(AudioInfo::FailedOpen(path.to_path_buf())).unwrap();
				}
			}
			Err(_) => {}
		}
	}

	fn pause(&mut self) {
		self.sink.pause();
	}

	fn unpause(&mut self) {
		self.sink.play();
	}
}
