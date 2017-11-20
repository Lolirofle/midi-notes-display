#![feature(slice_patterns,range_contains)]

extern crate core;
extern crate font_loader;
extern crate nom_midi as midi;
extern crate rusttype;
#[macro_use] extern crate conrod;

use conrod::widget;
use conrod::backend::glium::glium::{self,Surface};

mod filtered_scan_iter;
mod midi_ext;
mod pair_iter;

use midi_ext::*;

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

widget_ids!(struct Ids{
	canvas,
	tones_wrapper_canvas,
	tones_canvas,
	tones_grid,
	tones_scrollbar_x,
	tones_scrollbar_y,
	settings_canvas,
	settings_barwidth_slider,
	settings_barheight_slider,
	setting1_wrapper_canvas,
	setting2_wrapper_canvas,
});

struct Song{
	tones: Vec<Tone>,
	duration: u32,
}

fn main(){
	use std::{env,fs};
	use std::io::Read;

	//Constants
	const INITIAL_WIDTH: u32 = 800;
	const INITIAL_HEIGHT: u32 = 600;

	//MIDI file import
	let midi_data = {
		let mut midi_file_contents = Vec::new();
		let mut midi_file = fs::File::open(env::args().nth(1).expect("MIDI file path is unspecified. Expected a 1st command line argument.")).expect("Unable to open file specified from 1st command line argument.");
		midi_file.read_to_end(&mut midi_file_contents).expect("Unable to read file specified from 1st command line argument.");
		midi::parser::parse_midi(midi_file_contents.as_slice()).to_result().expect("Unable to parse file specified from 1st command line argument as MIDI file.")
	};
	let song = Song{
		tones   : midi_to_tones(&midi_data),
		duration: midi_duration(&midi_data),
	};

	//Build window
	let mut events_loop = glium::glutin::EventsLoop::new();
	let window = glium::glutin::WindowBuilder::new()
		.with_title("MIDI Notes Display")
		.with_dimensions(INITIAL_WIDTH,INITIAL_HEIGHT);
	let context = glium::glutin::ContextBuilder::new()
		.with_vsync(true);
	let display = glium::Display::new(window,context,&events_loop).expect("Unable to open/create display/window");

	//Construct UI object
	let mut ui = conrod::UiBuilder::new([INITIAL_WIDTH as f64,INITIAL_HEIGHT as f64]).build();

	//Generate unique widget identifiers
	let ids = Ids::new(ui.widget_id_generator());

	//Add a font to the UI's `font::Map`
	ui.fonts.insert(
		rusttype::FontCollection::from_bytes(
			font_loader::system_fonts::get(&font_loader::system_fonts::FontPropertyBuilder::new().family("DejaVu Sans").build())
			.or_else(|| font_loader::system_fonts::get(&font_loader::system_fonts::FontPropertyBuilder::new().family("Tahoma").build()))
			.or_else(|| font_loader::system_fonts::get(&font_loader::system_fonts::FontPropertyBuilder::new().family("sans-serif").build()))
			.unwrap().0
		)
			.into_font()
			.unwrap()
	);

	//Used for converting `conrod::render::Primitives` into `Command`s that can be used for drawing to the glium `Surface`
	let mut renderer = conrod::backend::glium::Renderer::new(&display).unwrap();

	//Image map describing every widget->image mapping.
	//There are none here.
	let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

	//Poll events from the window.
	let mut tone_widget_ids = Vec::new();
	let mut tone_widget_size = [1.0,16.0];
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
		set_ui(&mut ui.set_widgets(),&ids,&song,&mut tone_widget_ids,&mut tone_widget_size);

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
fn set_ui(ui: &mut conrod::UiCell,ids: &Ids,song: &Song,tone_widget_ids: &mut Vec<[widget::Id; 2]>,tone_widget_size: &mut [f64; 2]){
	use conrod::{color,widget,Borderable,Color,Colorable,Labelable,Positionable,Sizeable,Widget};
	use conrod::position::Scalar;
	use conrod::widget::grid;
	use core::iter;

	//Canvas widget
	//Contains everything.
	widget::Canvas::new()
		.flow_down(&[
			//Tones canvas wrapper widget
			(ids.tones_wrapper_canvas , widget::Canvas::new()
				.scroll_kids()
				.color(color::DARK_CHARCOAL)
			),

			//Tones canvas wrapper widget
			(ids.settings_canvas , widget::Canvas::new()
				.color(color::DARK_GRAY)
				.length(96.0)
				.border(6.0)
				.border_color(Color::Rgba(0.3 , 0.3 , 0.3 , 1.0))
				.flow_right(&[
					(ids.setting1_wrapper_canvas , widget::Canvas::new().color(color::TRANSPARENT)),
					(ids.setting2_wrapper_canvas , widget::Canvas::new().color(color::TRANSPARENT)),
				])
			),
		])
		.color(color::BLACK)
		.set(ids.canvas,ui);


	//Tones canvas widget
	//Contains tones, and have a fixed size based on the song duration and bar heights so that scrolling in tones_wrapper_canvas widget works.
	widget::Canvas::new()
		.parent(ids.tones_wrapper_canvas)
		.place_on_kid_area(true)
		.top_left()
		.color(color::TRANSPARENT)
		.pad(16.0)
		.wh([
			(song.duration as Scalar) * tone_widget_size[0],
			(NOTES as Scalar)         * tone_widget_size[1],
		])
		.set(ids.tones_canvas,ui);

	//Generate more tone bar widget ids if not enough
	if tone_widget_ids.len() < song.tones.len(){
		tone_widget_ids.reserve(song.tones.len());
		for _ in tone_widget_ids.len()..song.tones.len(){
			tone_widget_ids.push([
				ui.widget_id_generator().next(),
				ui.widget_id_generator().next(),
			]);
		}
	}

	//Tone bar widgets
	//If `rect_of` and `wh_of` returns None, then it is difficult to do many of the things here (The most important being hiding invisible tone bars).
	if let (Some(view_rect),Some([view_w,view_h])) = (ui.rect_of(ids.tones_canvas) , ui.wh_of(ids.canvas)){
		let view_x = -view_rect.x.start - view_w/2.0;
		let view_y =  view_rect.y.end   - view_h/2.0;

		//Notes grid
		widget::Grid::new(
			view_rect.x.start,
			view_rect.x.end,
			view_rect.y.start,
			view_rect.y.end,
			iter::once(grid::Axis::Y(grid::Lines::step(tone_widget_size[1]).thickness(1.0).color(Color::Rgba(0.5,0.5,0.5,0.1))))
		)
			.parent(ids.tones_canvas)
			.place_on_kid_area(true)
			.top_left_of(ids.tones_canvas)
			.set(ids.tones_grid,ui);

		for (tone,[bar_id,text_id]) in song.tones.iter().zip(tone_widget_ids.iter().cloned()){
			let x = (tone.start_time as Scalar) * tone_widget_size[0];
			let y = (Into::<u8>::into(tone.note) as f64) * tone_widget_size[1];
			let w = ((tone.end_time as Scalar)-(tone.start_time as Scalar)) * tone_widget_size[0];
			let h = tone_widget_size[1];

			//Hide invisible tone bars (those that are not inside the scrolled view)
			if x+w < view_x{continue} //Skip to the left.
			if x > view_x+view_w{break} //Skip to the right. Break is okay because `song.tones` is guaranteed to be sorted.

			if y+h < view_y{continue} //Skip above.
			if y > view_y+view_h{continue} //Skip below.

			//Bars widgets
			widget::Rectangle::fill_with([w,h],Color::Rgba(1.0,1.0,1.0,0.5))
				.parent(ids.tones_canvas)
				.place_on_kid_area(true)
				.top_left_with_margins_on(ids.tones_canvas,y,x)
				.set(bar_id,ui);

			//Bar note text widgets
			if w >= 20.0 && h >= 10.0{
				widget::Text::new(note_name(tone.note))
					.parent(bar_id)
					.graphics_for(bar_id)
					.font_size((tone_widget_size[1]*0.75) as u32)
					.color(color::BLACK)
					.middle_of(bar_id)
					.center_justify()
					.set(text_id,ui);
			}
		}
	}

	//Tones horizontal scrollbar
	widget::Scrollbar::x_axis(ids.tones_wrapper_canvas)
		.thickness(20.0)
		.auto_hide(false)
		.set(ids.tones_scrollbar_x,ui);

	//Tones vertical scrollbar
	widget::Scrollbar::y_axis(ids.tones_wrapper_canvas)
		.thickness(20.0)
		.auto_hide(false)
		.set(ids.tones_scrollbar_y,ui);

	//Settings bar width slider
	if let Some(value) = widget::Slider::new(tone_widget_size[0] , 0.01 , 8.0)
		.parent(ids.setting1_wrapper_canvas)
		.place_on_kid_area(true)
		.middle_of(ids.setting1_wrapper_canvas)
		.wh([140.0 , 16.0])
		.label("Bar width")
		.label_font_size(9)
		.skew(5.0)
		.enabled(true)
		.set(ids.settings_barwidth_slider,ui)
	{
		tone_widget_size[0] = value;
	}

	//Settings bar height slider
	if let Some(value) = widget::Slider::new(tone_widget_size[1] , 4.0 , 64.0)
		.parent(ids.setting2_wrapper_canvas)
		.place_on_kid_area(true)
		.middle_of(ids.setting2_wrapper_canvas)
		.wh([140.0 , 16.0])
		.label("Bar height")
		.label_font_size(9)
		.skew(5.0)
		.enabled(true)
		.set(ids.settings_barheight_slider,ui)
	{
		tone_widget_size[1] = value;
	}
}
