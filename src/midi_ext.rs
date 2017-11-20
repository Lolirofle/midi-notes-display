use midi;
use midi::note::Note;

use filtered_scan_iter::*;

#[derive(Copy,Clone,Debug,PartialEq)]
pub struct Tone{
	pub note      : Note,
	pub start_time: u32,
	pub end_time  : u32,
	pub atk_vel   : u8,
	pub rel_vel   : u8,
}
pub fn midi_to_tones(midi_data: &midi::Midi) -> Vec<Tone>{
	use core::u8;
	use midi::{MidiEvent,MidiEventType};

	midi_data
		.tracks
		.iter()
		.flat_map((|track| &track.events)/* as fn(&midi::Track) -> &Vec<midi::Event>*/)
		.filtered_scan(
			(0 , [None; (u8::MAX as usize)-(u8::MIN as usize)].to_vec()), //TODO: Is this conversion inefficient? The allocation certainly should be.
			(|&mut (ref mut time,ref mut notes_on) , &midi::Event{delta_time,ref event,..}|{
				*time+= delta_time;

				if let &midi::EventType::Midi(MidiEvent{event: midi_event_type,..}) = event{
					match midi_event_type{
						MidiEventType::NoteOn(note,atk_vel) => {
							let note_on = &mut notes_on[Into::<u8>::into(note) as usize];

							if note_on.is_none(){
								*note_on = Some((*time,atk_vel));
							}

							None
						},
						MidiEventType::NoteOff(note,rel_vel) => {
							let note_on = &mut notes_on[Into::<u8>::into(note) as usize];

							if let &mut Some((start_time,atk_vel)) = note_on{
								*note_on = None;
								Some(Tone{
									start_time: start_time,
									end_time  : *time,
									note      : note,
									atk_vel   : atk_vel,
									rel_vel   : rel_vel,
								})
							}else{
								None
							}
						},
						_ => None
					}
				}else{
					None
				}
			})// as fn(&mut (u32,Vec<Option<(u32,u8)>>),&midi::Event) -> Option<_>
		)
		.collect()
}

pub fn midi_duration(midi_data: &midi::Midi) -> u32{
	midi_data
		.tracks
		.iter()
		.flat_map((|track| &track.events))
		.fold(0 , |time , &midi::Event{delta_time,..}|{
			time + delta_time
		})
}

pub const NOTES: usize = 128;

pub fn note_name(note: Note) -> &'static str{use self::Note::*; match note{
    C2n => "C₋₂",
    Cs2n=> "C♯₋₂",
    D2n => "D₋₂",
    Ds2n=> "D♯₋₂",
    E2n => "E₋₂",
    F2n => "F₋₂",
    Fs2n=> "F♯₋₂",
    G2n => "G₋₂",
    Gs2n=> "G♯₋₂",
    A1n => "A₋₁",
    As1n=> "A♯₋₁",
    B1n => "B₋₁",
    C1n => "C₋₁",
    Cs1n=> "C♯₋₁",
    D1n => "D₋₁",
    Ds1n=> "D♯₋₁",
    E1n => "E₋₁",
    F1n => "F₋₁",
    Fs1n=> "F♯₋₁",
    G1n => "G₋₁",
    Gs1n=> "G♯₋₁",
    A0  => "A₀",
    As0 => "A♯₀",
    B0  => "B₀",
    C0  => "C₀",
    Cs0 => "C♯₀",
    D0  => "D₀",
    Ds0 => "D♯₀",
    E0  => "E₀",
    F0  => "F₀",
    Fs0 => "F♯₀",
    G0  => "G₀",
    Gs0 => "G♯₀",
    A1  => "A₁",
    As1 => "A♯₁",
    B1  => "B₁",
    C1  => "C₁",
    Cs1 => "C♯₁",
    D1  => "D₁",
    Ds1 => "D♯₁",
    E1  => "E₁",
    F1  => "F₁",
    Fs1 => "F♯₁",
    G1  => "G₁",
    Gs1 => "G♯₁",
    A2  => "A₂",
    As2 => "A♯₂",
    B2  => "B₂",
    C2  => "C₂",
    Cs2 => "C♯₂",
    D2  => "D₂",
    Ds2 => "D♯₂",
    E2  => "E₂",
    F2  => "F₂",
    Fs2 => "F♯₂",
    G2  => "G₂",
    Gs2 => "G♯₂",
    A3  => "A₃",
    As3 => "A♯₃",
    B3  => "B₃",
    C3  => "C₃",
    Cs3 => "C♯₃",
    D3  => "D₃",
    Ds3 => "D♯₃",
    E3  => "E₃",
    F3  => "F₃",
    Fs3 => "F♯₃",
    G3  => "G₃",
    Gs3 => "G♯₃",
    A4  => "A₄",
    As4 => "A♯₄",
    B4  => "B₄",
    C4  => "C₄",
    Cs4 => "C♯₄",
    D4  => "D₄",
    Ds4 => "D♯₄",
    E4  => "E₄",
    F4  => "F₄",
    Fs4 => "F♯₄",
    G4  => "G₄",
    Gs4 => "G♯₄",
    A5  => "A₅",
    As5 => "A♯₅",
    B5  => "B₅",
    C5  => "C₅",
    Cs5 => "C♯₅",
    D5  => "D₅",
    Ds5 => "D♯₅",
    E5  => "E₅",
    F5  => "F₅",
    Fs5 => "F♯₅",
    G5  => "G₅",
    Gs5 => "G♯₅",
    A6  => "A₆",
    As6 => "A♯₆",
    B6  => "B₆",
    C6  => "C₆",
    Cs6 => "C♯₆",
    D6  => "D₆",
    Ds6 => "D♯₆",
    E6  => "E₆",
    F6  => "F₆",
    Fs6 => "F♯₆",
    G6  => "G₆",
    Gs6 => "G♯₆",
    A7  => "A₇",
    As7 => "A♯₇",
    B7  => "B₇",
    C7  => "C₇",
    Cs7 => "C♯₇",
    D7  => "D₇",
    Ds7 => "D♯₇",
    E7  => "E₇",
    F7  => "F₇",
    Fs7 => "F♯₇",
    G7  => "G₇",
    Gs7 => "G♯₇",
    A8  => "A₈",
    As8 => "A♯₈",
    B8  => "B₈",
    C8  => "C₈",
    Cs8 => "C♯₈",
    D8  => "D₈",
    Ds8 => "D♯₈",
    E8  => "E₈",
    F8  => "F₈",
    Fs8 => "F♯₈",
    G8  => "G₈",
}}
