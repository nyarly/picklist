use druid::widget::{Widget, RawLabel, Controller,  ViewSwitcher };
use druid::{
  AppLauncher,
  Color,
  Data,
  WidgetExt,
  WindowDesc,
  Lens,
  lens,
  LensExt,
  EventCtx,
  Event,
  Env,
};
use druid::im::Vector;
use druid::text::RichText;
use std::rc::Rc;
use tracing::{debug,instrument};

use picklist::controllers::TakeFocus;

#[derive(Clone, Data, Lens, Debug)]
struct AppData {
  list: Vector<Rc<String>>,
  selected: Option<Rc<String>>
}

// TODO
// 1. Nicer layout - nicer "selected" appearance
// 2. Mouse events - click to select;
// 3. ? double click to select & exit...
// 4. Build for Linux/Mac/Win
fn main() -> Result<(), anyhow::Error> {
  let list_widget = picklist::picklist( || {
    ViewSwitcher::new(
      |data: &(Option<Rc<String>>, (Rc<String>, RichText)), _env: &_|
        if let (Some(sel), (item, _)) = data { sel == item } else { false },
      |selected: &bool, _data: &(Option<Rc<String>>, (Rc<String>, RichText)), _env: &_| {

        if *selected {
          RawLabel::new()
            .with_text_color(Color::BLACK)
            .background(Color::SILVER)
            .expand_width()
            .lens(lens!((_,_),1).then(lens!((_,_),1)))
            .on_click(|ctx, data, env| {
            })
            .boxed()
        }
        else {
          RawLabel::new()
            .expand_width()
            .lens(lens!((_,_),1).then(lens!((_,_),1)))
            .on_click(|ctx, d: &mut (Option<Rc<String>>, (Rc<String>, RichText)), _env| {
              let (sel, (item, _)) = d;
              debug!("Clicked! Item: {}", item);
              *sel = Some(item.clone());
              debug!("Updated! Sel: {:?}", sel);
            })
            .boxed()
        }
      }
    )
  }).lens((AppData::selected, AppData::list))
    .controller(DoneKeys{})
    .controller(EscClears{})
    .controller(TakeFocus{});

  let main_window = WindowDesc::new(list_widget);
  let list = std::io::stdin().lines().try_fold(vec![], |mut vec, line| {
    vec.push(Rc::new(line?));
    Ok::<_,std::io::Error>(vec)
  })?;
  let data = AppData{
    selected: None,
    list: list.into()
  };

  let mut launcher = AppLauncher::with_window(main_window);

  if let Ok(val) = std::env::var("DEBUG") {
    if val == "true" {
      launcher = launcher.log_to_console(); // XXX if env DEBUG=true
    }
  }

  launcher.launch(data)?;

  Ok(())
}

struct EscClears;

impl<W: Widget<AppData>> Controller<AppData, W> for EscClears {
  #[instrument(name="EscClears",skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut AppData, env: &Env) {
    use druid::{KeyEvent,KbKey};

    match event {
      Event::KeyUp(KeyEvent{key: KbKey::Escape, ..}) => {
        debug!("Clearing data"); data.selected = None
      },
      _ => ()
    }
    child.event(ctx, event, data, env);
  }
}

pub struct DoneKeys;

impl<W: Widget<AppData>> Controller<AppData, W> for DoneKeys {
  #[instrument(name="DoneKeys",skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut AppData, env: &Env) {
    use druid::{KeyEvent,KbKey,Application};


    child.event(ctx, event, data, env);
    match event {
      Event::KeyUp(KeyEvent{key: KbKey::Enter | KbKey::Escape, ..}) => {
        debug!( "{:?}", event);
        if let Some(val) = data.selected.clone() {
          println!("{}", val);
        }
        Application::global().quit()
      },
      _ => ()
    }
  }
}
