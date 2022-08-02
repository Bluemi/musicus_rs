use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam::{bounded, Receiver, Sender};
use rodio::{cpal, Decoder, DeviceTrait, Sink, Source, OutputStream};
use rodio::cpal::traits::HostTrait;

use crate::audio_backend::chunk::{CHUNK_SIZE, duration_to_position, position_to_duration, SamplesChunk};
use crate::audio_backend::receiver_source::ReceiverSource;
use crate::musicus::log;
use crate::song::{Song, SongID};

mod receiver_source;
mod chunk;

const CHUNK_BUFFER_SIZE: usize = 4;

pub struct AudioBackend {
	sink: Sink,
	_stream: OutputStream,

	/// sender for info to musicus
    info_sender: Sender<AudioInfo>,
	/// sender to source
	source_chunk_sender: Sender<SamplesChunk>,
	/// sender to audio backend, used to create loader threads
	audio_backend_sender: Sender<AudioBackendCommand>,

	current_song: Option<CurrentSongState>,
	next_song: Option<(Song, AudioSong)>,
	volume: f32,
}

struct AudioSong {
	song_id: SongID,
	chunks: Vec<SamplesChunk>,
	sample_rate_and_channels: Option<(u32, u16)>,
}

impl AudioSong {
	fn new(song_id: SongID) -> AudioSong {
		AudioSong {
			song_id,
			chunks: Vec::new(),
			sample_rate_and_channels: None,
		}
	}
}

struct CurrentSongState {
	play_position: usize, // the number of samples already sent to source. A sample is one f32 value.
	audio_song: AudioSong,
}

pub enum AudioCommand {
	Play(Song),
	Queue(Song),
	Pause,
    Unpause,
	Seek(SeekCommand),
    SetVolume(f32),
}

impl Debug for AudioCommand {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			AudioCommand::Play(song) => f.debug_struct("AudioCommand::Play").field("song", &song.get_id()).finish(),
			AudioCommand::Queue(song) => f.debug_struct("AudioCommand::Queue").field("song", &song.get_id()).finish(),
			AudioCommand::Pause => f.debug_struct("AudioCommand::Pause").finish(),
			AudioCommand::Unpause => f.debug_struct("AudioCommand::Unpause").finish(),
			AudioCommand::Seek(_) => f.debug_struct("AudioCommand::Seek").finish(),
			AudioCommand::SetVolume(volume) => f.debug_struct("AudioCommand::SetVolume").field("volume", volume).finish(),
		}
	}
}

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
pub enum SeekDirection {
	Forward,
	Backward,
}

#[derive(Debug)]
pub enum AudioInfo {
	Playing(SongID, Duration), // playing song, play duration
	SongStarts(SongID),
	FailedOpen(SongID, OpenError),
	SongDuration(SongID, Duration),
}

#[derive(Debug)]
pub struct PlayingUpdate {
	song_id: SongID,
	samples_played: usize,
}

#[derive(Debug)]
pub enum AudioUpdate {
	Playing(PlayingUpdate),
	SongStarts(SongID),
}

pub enum AudioBackendCommand {
	Command(AudioCommand), // commands from musicus
	Update(AudioUpdate), // update from source
	LoadInfo(LoadInfo), // chunk from loader
}


#[derive(Debug)]
pub enum LoadInfo {
	Chunk(SamplesChunk),
	Duration(SongID, Duration),
	Err(SongID, OpenError),
}

#[derive(Debug)]
pub enum OpenError {
	FileNotFound,
	NotDecodable,
}

impl AudioBackend {
	pub fn new(info_sender: Sender<AudioInfo>, audio_backend_sender: Sender<AudioBackendCommand>, volume: f32) -> AudioBackend {
		// sink and devices
		let pulse_device = cpal::default_host().output_devices().unwrap().find(|d| d.name().unwrap().contains("pulse")).unwrap(); // TODO: dont force pulse device
		let (stream, stream_handle) = OutputStream::try_from_device(&pulse_device)
			.unwrap_or_else(|_| OutputStream::try_default().unwrap());

		let sink = Sink::try_new(&stream_handle).unwrap();

		// receiver source
		let (source_chunk_sender, chunk_receiver) = bounded(CHUNK_BUFFER_SIZE);
		let receiver_source = ReceiverSource::new(chunk_receiver, audio_backend_sender.clone());

		sink.append(receiver_source);
		sink.play();
		sink.set_volume(volume);

		AudioBackend {
			sink,
			_stream: stream,

			info_sender,
			source_chunk_sender,
			audio_backend_sender,

			current_song: None,
			next_song: None,
			volume,
		}
	}

	pub fn run(&mut self, audio_backend_receiver: Receiver<AudioBackendCommand>) {
		while let Ok(command) = audio_backend_receiver.recv() {
			let mut commands = vec![command];
			commands.extend(audio_backend_receiver.try_iter());
			let commands = AudioBackendCommand::simplify(commands);
			for command in commands.into_iter() {
				match command {
					AudioBackendCommand::Command(command) => self.handle_command(command),
					AudioBackendCommand::Update(update) => self.handle_update(update),
					AudioBackendCommand::LoadInfo(load_info) => self.handle_load_info(load_info),
				}
			}
		}
	}

	fn handle_command(&mut self, command: AudioCommand) {
		match command {
			AudioCommand::Play(song) => self.play(song),
			AudioCommand::Queue(song) => self.queue(song),
			AudioCommand::Pause => self.pause(),
			AudioCommand::Unpause => self.unpause(),
			AudioCommand::Seek(seek_command) => self.seek(seek_command),
			AudioCommand::SetVolume(volume) => self.set_volume(volume),
		}
	}

	/// Tries to send the next chunks to source
	fn send_next_chunks(&mut self) { // TODO: why is this called so often?
		loop {
			let current_song = match &mut self.current_song {
				Some(x) => x,
				None => break,
			};


			let next_chunk_index = current_song.play_position / CHUNK_SIZE + 1;
			match current_song.audio_song.chunks.get(next_chunk_index) {
				Some(chunk) => {
					match self.source_chunk_sender.try_send(chunk.clone()) {
						Ok(_) => {
							// we can use CHUNK_SIZE here, as play_position will be set to 0 if this is last_chunk and length != CHUNK_SIZE
							current_song.play_position += CHUNK_SIZE;
						}
						Err(crossbeam::TrySendError::Full(_)) => {
							break; // channel is full -> stop to try sending chunks
						}
						Err(_) => {
							todo!()
						}
					}
					if chunk.last_chunk {
						// we have completed the current song -> switch to next song
						Self::play_next_song(&mut self.current_song, &mut self.next_song);
					}
				}
				None => {
					// is last chunk in chunks? This would mean we are already past the last chunk (can happen by seeking)
					if current_song.audio_song.chunks.last().map(|c| c.last_chunk).unwrap_or(false) {
						Self::play_next_song(&mut self.current_song, &mut self.next_song);
					} else {
						// we have to wait for further chunks
						break;
					}
				}
			}
		}
	}

	fn play_next_song(current_song: &mut Option<CurrentSongState>, next_song: &mut Option<(Song, AudioSong)>) {
		if let Some(next_song) = next_song.take() {
			*current_song = Some(CurrentSongState {
				play_position: 0,
				audio_song: next_song.1
			});
		} else {
			*current_song = None;
		}
	}

	// TODO: This is probably not the best implementation
	fn is_song_loading(&self, song_id: SongID) -> bool {
		if let Some(current_song) = &self.current_song {
			if current_song.audio_song.song_id == song_id {
				return true;
			}
		}
		if let Some(next_audio_song) = &self.next_song {
			if next_audio_song.1.song_id == song_id {
				return true;
			}
		}
		false
	}

	fn load(&mut self, song: Song) {
		if !self.is_song_loading(song.get_id()) {
			let abs = self.audio_backend_sender.clone();
			thread::Builder::new().name("loader".to_string()).spawn(move || {
				load_chunks(song, abs.clone());
			}).expect("Failed to spawn loader thread");
		}
	}

	fn play(&mut self, song: Song) {
		self.load(song.clone());
		self.current_song = Some(CurrentSongState {
			audio_song: AudioSong::new(song.get_id()),
			play_position: 0,
		});
		self.send_next_chunks();
		self.sink.play();
	}

	fn queue(&mut self, song: Song) {
		self.load(song.clone());
		self.next_song = Some((song.clone(), AudioSong::new(song.get_id())));
	}

	#[allow(unused)]
	fn log_state(&self) {
		log(&format!(
			"update:\n\tcurrent song {}: {}/{} chunks\n\tnext song {}: {} chunks\n\tsend_next_chunk() was called: {}",
			self.current_song.as_ref().map_or(String::from("None"), |s| s.audio_song.song_id.to_string()),
			self.current_song.as_ref().map_or(String::from("None"), |s| (s.play_position / CHUNK_SIZE).to_string()),
			self.current_song.as_ref().map_or(String::from("None"), |s| s.audio_song.chunks.len().to_string()),
			self.next_song.as_ref().map_or(String::from("None"), |s| s.0.get_id().to_string()),
			self.next_song.as_ref().map_or(String::from("None"), |s| s.1.chunks.len().to_string()),
		));
	}

	fn set_volume(&mut self, volume: f32) {
		self.sink.set_volume(volume);
		self.volume = volume;
	}

	fn get_audio_song<'a>(current_song: Option<&'a mut CurrentSongState>, next_song: Option<&'a mut (Song, AudioSong)>, song_id: SongID) -> Option<&'a mut AudioSong> {
		current_song.map(|ca| &mut ca.audio_song).filter(|audio_song| audio_song.song_id == song_id)
		.or_else(|| next_song.map(|na| &mut na.1).filter(|audio_song| audio_song.song_id == song_id))
	}

	fn handle_update(&mut self, update: AudioUpdate) {
		match update {
			AudioUpdate::Playing(playing_update) => {
				let audio_song = Self::get_audio_song(self.current_song.as_mut(), self.next_song.as_mut(), playing_update.song_id);
				if let Some(audio_song) = audio_song {
					if let Some((sample_rate, channels)) = audio_song.sample_rate_and_channels {
						let duration = position_to_duration(playing_update.samples_played, sample_rate, channels);
						self.info_sender.send(AudioInfo::Playing(playing_update.song_id, duration)).unwrap();
					}
				}
				self.send_next_chunks();
			}
			AudioUpdate::SongStarts(song_id) => {
				self.info_sender.send(AudioInfo::SongStarts(song_id)).unwrap();
			}
		}
	}

	fn handle_load_info(&mut self, load_info: LoadInfo) {
		match load_info {
			LoadInfo::Chunk(chunk) => {
				if let Some(audio_song) = Self::get_audio_song(self.current_song.as_mut(), self.next_song.as_mut(), chunk.song_id) {
					if audio_song.sample_rate_and_channels.is_none() {
						audio_song.sample_rate_and_channels = Some((chunk.sample_rate, chunk.channels));
					}
					audio_song.chunks.push(chunk);
					self.send_next_chunks();
				}
			}
			LoadInfo::Duration(song_id, duration) => {
				let _ = self.info_sender.send(AudioInfo::SongDuration(song_id, duration)); // TODO: handle error
			}
			LoadInfo::Err(song, e) => {
				let _ = self.info_sender.send(AudioInfo::FailedOpen(song, e)); // TODO: handle error
			}
		}
	}

	fn seek(&mut self, seek_command: SeekCommand) {
		if let Some(current_song) = &mut self.current_song {
			if let Some((sample_rate, channels)) = current_song.audio_song.sample_rate_and_channels {
				let offset = duration_to_position(&seek_command.duration, sample_rate, channels);
				match seek_command.direction {
					SeekDirection::Forward => {
						current_song.play_position += offset;
					}
					SeekDirection::Backward => {
						current_song.play_position = current_song.play_position.checked_sub(offset).unwrap_or(0);
					}
				}
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

/**
 * Loads chunks of the given song
 */
fn load_chunks(song: Song, chunk_sender: Sender<AudioBackendCommand>) {
	if let Ok(file) = File::open(&song.get_path()) {
		if let Ok(decoder) = Decoder::new(BufReader::new(file)) {
			let channels = decoder.channels();
			let sample_rate = decoder.sample_rate();
			let total_duration = decoder.total_duration();

			if let Some(duration) = total_duration {
				let _ = chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Duration(song.get_id(), duration)));
			}

			let mut data = Box::new([0.0f32; CHUNK_SIZE]);
			let mut index = 0;
			let mut next_start_position = 0;
			let mut converted = decoder.convert_samples().peekable();

			while let Some(sample) = converted.next() {
				let chunk_index = index % CHUNK_SIZE;
				data[chunk_index] = sample;

				// send chunk
				if chunk_index == CHUNK_SIZE-1 {
					let last_chunk = converted.peek().is_none();
					let chunk = SamplesChunk {
						channels,
						sample_rate,
						start_position: next_start_position,
						length: CHUNK_SIZE,
						data: Arc::from(data.clone()),
						song_id: song.get_id(),
						last_chunk,
					};
					// calculate duration, if not already done
					if last_chunk && total_duration.is_none() {
						let duration = position_to_duration(next_start_position + CHUNK_SIZE, sample_rate, channels);
						let _ = chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Duration(song.get_id(), duration)));
					}
					next_start_position = index + 1;
					if chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Chunk(chunk))).is_err() {
						return;
					}
				}
				index += 1;
			}
			let chunk_index = index % CHUNK_SIZE;
			if chunk_index != 0 {
				let chunk = SamplesChunk {
					channels,
					sample_rate,
					start_position: next_start_position,
					length: index - next_start_position,
					data: Arc::from(data),
					song_id: song.get_id(),
					last_chunk: true,
				};
				if total_duration.is_none() {
					let duration = position_to_duration(next_start_position + CHUNK_SIZE, sample_rate, channels);
					let _ = chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Duration(song.get_id(), duration)));
				}
				if chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Chunk(chunk))).is_err() {
					return;
				}
			}
		} else {
			let _ = chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Err(song.get_id(), OpenError::NotDecodable)));
		}
	} else {
		let _ = chunk_sender.send(AudioBackendCommand::LoadInfo(LoadInfo::Err(song.get_id(), OpenError::FileNotFound)));
	}
}

impl AudioBackendCommand {
	pub fn simplify(vec: Vec<AudioBackendCommand>) -> Vec<AudioBackendCommand> {
		let mut result = Vec::new();
		let mut last_play_command = None;
		let mut last_playing_update = None;
		let mut seek_command: Option<SeekCommand> = None;
		let mut last_set_volume: Option<f32> = None;
		let mut load_infos = Vec::new();

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
				li @ AudioBackendCommand::LoadInfo(_) => {
					load_infos.push(li);
				}
			}
		}
		result.append(&mut load_infos);
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
