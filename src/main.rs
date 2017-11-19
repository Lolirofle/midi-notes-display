#![feature(conservative_impl_trait)]

extern crate core;
extern crate nom_midi as midi;
#[macro_use] extern crate conrod;

use conrod::widget;
use conrod::backend::glium::glium::{self,Surface};
use core::u8;

mod filtered_scan_iter;
mod pair_iter;

use filtered_scan_iter::*;

///In most of the examples the `glutin` crate is used for providing the window context and
///events while the `glium` crate is used for displaying `conrod::render::Primitives` to the
///screen.
///
///This `Iterator`-like type simplifies some of the boilerplate involved in setting up a
///glutin+glium event loop that works efficiently with conrod.
pub struct EventLoop{
	ui_needs_update: bool,
	last_update: std::time::Instant,
}

impl EventLoop{
	pub fn new() -> Self{
		EventLoop{
			last_update: std::time::Instant::now(),
			ui_needs_update: true,
		}
	}

	///Produce an iterator yielding all available events.
	pub fn next(&mut self,events_loop: &mut glium::glutin::EventsLoop) -> Vec<glium::glutin::Event>{
		//We don't want to loop any faster than 60 FPS, so wait until it has been at least 16ms
		//since the last yield.
		let last_update = self.last_update;
		let sixteen_ms = std::time::Duration::from_millis(16);
		let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
		if duration_since_last_update < sixteen_ms{
			std::thread::sleep(sixteen_ms - duration_since_last_update);
		}

		//Collect all pending events.
		let mut events = Vec::new();
		events_loop.poll_events(|event| events.push(event));

		//If there are no events and the `Ui` does not need updating, wait for the next event.
		if events.is_empty() && !self.ui_needs_update{
			events_loop.run_forever(|event|{
				events.push(event);
				glium::glutin::ControlFlow::Break
			});
		}

		self.ui_needs_update = false;
		self.last_update = std::time::Instant::now();

		events
	}

	///Notifies the event loop that the `Ui` requires another update whether or not there are any
	///pending events.
	///
	///This is primarily used on the occasion that some part of the `Ui` is still animating and
	///requires further updates to do so.
	pub fn needs_update(&mut self){
		self.ui_needs_update = true;
	}

}

#[derive(Copy,Clone,Debug,PartialEq)]
struct Tone{
	pub note      : midi::note::Note,
	pub start_time: u32,
	pub end_time  : u32,
	pub atk_vel   : u8,
	pub rel_vel   : u8,
}
fn midi_to_tones(midi_data: &midi::Midi) -> Vec<Tone>{
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

widget_ids!(struct Ids{
	canvas,
	triangles,
	triangles_scrollbar_x,
	triangles_scrollbar_y,
});

fn main(){
	//Constants
	const INITIAL_WIDTH: u32 = 800;
	const INITIAL_HEIGHT: u32 = 600;
	const FONT_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"),"/test.ttf");

	//MIDI file import
	let tones = {
		let data = include_bytes!("../test.mid");
		let midi_data = midi::parser::parse_midi(data).unwrap().1;
		midi_to_tones(&midi_data)
	};

	//Build window
	let mut events_loop = glium::glutin::EventsLoop::new();
	let window = glium::glutin::WindowBuilder::new()
		.with_title("MIDI Notes Display")
		.with_dimensions(INITIAL_WIDTH,INITIAL_HEIGHT);
	let context = glium::glutin::ContextBuilder::new()
		.with_vsync(true);
	let display = glium::Display::new(window,context,&events_loop).unwrap();

	//Construct UI object
	let mut ui = conrod::UiBuilder::new([INITIAL_WIDTH as f64,INITIAL_HEIGHT as f64]).build();

	//Generate unique widget identifiers
	let ids = Ids::new(ui.widget_id_generator());

	//Add a font to the UI's `font::Map`
	ui.fonts.insert_from_file(FONT_PATH).unwrap();

	//Used for converting `conrod::render::Primitives` into `Command`s that can be used for drawing to the glium `Surface`
	let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

	//Image map describing every widget->image mapping.
	//There are none here.
	let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

	//Poll events from the window.
	let mut tone_widget_ids = Vec::new();
	let mut event_loop = EventLoop::new();
	'main: loop{
		//Handle all events
		for event in event_loop.next(&mut events_loop){
			//Use `winit` backend to convert winit events to conrod events
			if let Some(event) = conrod::backend::winit::convert_event(event.clone(),&display){
				ui.handle_event(event);
				event_loop.needs_update();
			}

			match event{
				glium::glutin::Event::WindowEvent{event,..} => match event{
					//Closing application
					glium::glutin::WindowEvent::Closed |
					glium::glutin::WindowEvent::KeyboardInput {
						input: glium::glutin::KeyboardInput {
							virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
							..
						},
						..
					} => break 'main,
					_ => (),
				},
				_ => (),
			}
		}

		//Initiate widgets
		set_ui(&mut ui.set_widgets(),&ids,&tones,&mut tone_widget_ids,[1.0,16.0]);

		//Render GUI when something has changed
		if let Some(primitives) = ui.draw_if_changed(){
			renderer.fill(&display,primitives,&image_map);
			let mut target = display.draw();
			target.clear_color(0.0 , 0.0 , 0.0 , 1.0);
			renderer.draw(&display,&mut target,&image_map).unwrap();
			target.finish().unwrap();
		}
	}
}

//Set the widgets
fn set_ui(ui: &mut conrod::UiCell,ids: &Ids,tones: &Vec<Tone>,tone_widget_ids: &mut Vec<widget::Id>,tone_widget_size: [f64; 2]){
	use conrod::{color,widget,Color,Colorable,Positionable,Sizeable,Widget};
	use conrod::position::{Position,Scalar};

	//Canvas
	widget::Canvas::new()
		.scroll_kids()
		.color(color::DARK_CHARCOAL)
		.set(ids.canvas,ui);

	//Tone bars
	if tone_widget_ids.len() < tones.len(){
		tone_widget_ids.reserve(tones.len());
		for _ in tone_widget_ids.len()..tones.len(){
			tone_widget_ids.push(ui.widget_id_generator().next());
		}
	}

	for (tone,id) in tones.iter().zip(tone_widget_ids.iter().cloned()){
		widget::Rectangle::fill_with([((tone.end_time as Scalar)-(tone.start_time as Scalar))*tone_widget_size[0] , tone_widget_size[1]] ,Color::Rgba(1.0,1.0,1.0,0.5))
			.parent(ids.canvas)
			.place_on_kid_area(true)
			.xy([
				(tone.start_time as Scalar) * tone_widget_size[0],
				(Into::<u8>::into(tone.note) as f64) * tone_widget_size[1],
			])
			.set(id,ui);
	}

	//Horizontal scrollbar
	widget::Scrollbar::x_axis(ids.canvas)
		.thickness(20.0)
		.auto_hide(false)
		.set(ids.triangles_scrollbar_x,ui);

	//Vertical scrollbar
	widget::Scrollbar::y_axis(ids.canvas)
		.thickness(20.0)
		.auto_hide(false)
		.set(ids.triangles_scrollbar_y,ui);
}
