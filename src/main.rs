extern crate core;
extern crate nom_midi as midi;
#[macro_use] extern crate conrod;

use conrod::{color, widget, Widget};
use conrod::widget::triangles::Triangle;
use conrod::backend::glium::glium::{self, Surface};
use core::u8;
use midi::{Event,EventType,MidiEvent,MidiEventType};

mod pair_iter;
use pair_iter::*;

///In most of the examples the `glutin` crate is used for providing the window context and
///events while the `glium` crate is used for displaying `conrod::render::Primitives` to the
///screen.
///
///This `Iterator`-like type simplifies some of the boilerplate involved in setting up a
///glutin+glium event loop that works efficiently with conrod.
pub struct EventLoop {
	ui_needs_update: bool,
	last_update: std::time::Instant,
}

impl EventLoop {
	pub fn new() -> Self {
		EventLoop {
			last_update: std::time::Instant::now(),
			ui_needs_update: true,
		}
	}

	///Produce an iterator yielding all available events.
	pub fn next(&mut self, events_loop: &mut glium::glutin::EventsLoop) -> Vec<glium::glutin::Event> {
		//We don't want to loop any faster than 60 FPS, so wait until it has been at least 16ms
		//since the last yield.
		let last_update = self.last_update;
		let sixteen_ms = std::time::Duration::from_millis(16);
		let duration_since_last_update = std::time::Instant::now().duration_since(last_update);
		if duration_since_last_update < sixteen_ms {
			std::thread::sleep(sixteen_ms - duration_since_last_update);
		}

		//Collect all pending events.
		let mut events = Vec::new();
		events_loop.poll_events(|event| events.push(event));

		//If there are no events and the `Ui` does not need updating, wait for the next event.
		if events.is_empty() && !self.ui_needs_update {
			events_loop.run_forever(|event| {
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
	pub fn needs_update(&mut self) {
		self.ui_needs_update = true;
	}

}

fn main(){
	//Constants
	const WIDTH: u32 = 700;
	const HEIGHT: u32 = 400;
	const FONT_PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/test.ttf");

	//MIDI
	let data = include_bytes!("../test.mid");
	let midi_data = midi::parser::parse_midi(data).unwrap().1;
	//println!("{:#?}",midi_data);

	//Build the window.
	let mut events_loop = glium::glutin::EventsLoop::new();
	let window = glium::glutin::WindowBuilder::new()
		.with_title("Triangles!")
		.with_dimensions(WIDTH, HEIGHT);
	let context = glium::glutin::ContextBuilder::new()
		.with_vsync(true)
		.with_multisampling(4);
	let display = glium::Display::new(window, context, &events_loop).unwrap();

	//construct our `Ui`.
	let mut ui = conrod::UiBuilder::new([WIDTH as f64, HEIGHT as f64]).build();

	//Generate the widget identifiers.
	widget_ids!(struct Ids{
		triangles,
		triangles_scrollbar_x,
		triangles_scrollbar_y,
	});
	let ids = Ids::new(ui.widget_id_generator());

	//Add a `Font` to the `Ui`'s `font::Map` from file.
	ui.fonts.insert_from_file(FONT_PATH).unwrap();

	//A type used for converting `conrod::render::Primitives` into `Command`s that can be used
	//for drawing to the glium `Surface`.
	let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

	//The image map describing each of our widget->image mappings (in our case, none).
	let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

	events_loop.run_forever(|event| {

		match event.clone() {
			glium::glutin::Event::WindowEvent { event, .. } => match event {

				//Break from the loop upon `Escape` or closed window.
				glium::glutin::WindowEvent::Closed |
				glium::glutin::WindowEvent::KeyboardInput {
					input: glium::glutin::KeyboardInput {
						virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
						..
					},
					..
				} => return glium::glutin::ControlFlow::Break,

				_ => (),
			},
			_ => (),
		}

		//Use the `winit` backend feature to convert the winit event to a conrod one.
		let input = match conrod::backend::winit::convert_event(event, &display) {
			None => return glium::glutin::ControlFlow::Continue,
			Some(input) => input,
		};

		//Handle the input with the `Ui`.
		ui.handle_event(input);

		//Set the triangle widget.
		{
			let ui = &mut ui.set_widgets();
			let window_rect = ui.rect_of(ui.window).unwrap(); //window_rect.l_r_b_t()
			let color = color::WHITE.to_rgb();

			let mut time: u32 = 0;
			let mut notes_on: Vec<Option<u32>> = [None; (u8::MAX as usize)-(u8::MIN as usize)].to_vec(); //TODO: Is this conversion inefficient?
			widget::Triangles::multi_color(midi_data
				.tracks
				.iter()
				.flat_map(|track| &track.events)
				.flat_map(|&Event{delta_time,ref event,..}|{
					time+= delta_time;

					if let &EventType::Midi(MidiEvent{event: midi_event_type,..}) = event{
						match midi_event_type{
							MidiEventType::NoteOn (note,_) => {
								let note_on = &mut notes_on[Into::<u8>::into(note) as usize];

								if note_on.is_none(){
									*note_on = Some(time);
								}

								PairIter::from([])
							},
							MidiEventType::NoteOff(note,_) => {
								let note_on = &mut notes_on[Into::<u8>::into(note) as usize];

								const NOTE_HEIGHT: f64 = 4.0;

								if let &mut Some(start_time) = note_on{
									*note_on = None;

									let (l,r,b,t) = (
										start_time as f64,
										time as f64,
										((Into::<u8>::into(note)+1) as f64)*NOTE_HEIGHT,
										(Into::<u8>::into(note) as f64)*NOTE_HEIGHT
									);

									PairIter::from([
										Triangle([([l,b], color), ([l,t], color), ([r,t], color)]),
										Triangle([([r,t], color), ([r,b], color), ([l,b], color)]),
									])
								}else{
									PairIter::from([])
								}
							},
							_ => PairIter::from([])
						}
					}else{
						PairIter::from([])
					}
				}
			))
				.with_bounding_rect(window_rect)
				.set(ids.triangles, ui);
		}

		//Draw the `Ui` if it has changed.
		if let Some(primitives) = ui.draw_if_changed() {
			renderer.fill(&display, primitives, &image_map);
			let mut target = display.draw();
			target.clear_color(0.0, 0.0, 0.0, 1.0);
			renderer.draw(&display, &mut target, &image_map).unwrap();
			target.finish().unwrap();
		}

		glium::glutin::ControlFlow::Continue
	});
}
