use druid::widget::{Widget, Label, Controller,  ViewSwitcher };
use druid::{
  AppLauncher,
  Color,
  Data,
  WidgetExt,
  WindowDesc,
  Lens,
  EventCtx,
  Event,
  Env,
};
use druid::im::Vector;
use std::rc::Rc;
use tracing::{span,  Level, instrument};

use picklist::controllers::{DoneKeys,TakeFocus};

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
      |data: &(Option<Rc<String>>, Rc<String>), _env: &_|
        if let (Some(sel), item) = data { sel == item } else { false },
      |selected: &bool, _data: &(Option<Rc<String>>, Rc<String>), _env: &_| {
        let label = Label::new(|(_, item): &(Option<Rc<String>>, Rc<String>), _env: &_| (**item).clone())
          .expand_width();

        if *selected { label.border(Color::GREEN, 2.0).boxed() }
        else { label.border(Color::BLUE, 2.0).boxed() }
      }
    )
  }).lens((AppData::selected, AppData::list))
    .controller(DoneKeys{})
    .controller(EnterPrints{})
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

  Ok(AppLauncher::with_window(main_window)
    //.log_to_console() // XXX if env DEBUG=true
    .launch(data)?)
}

struct EnterPrints;

impl<W: Widget<AppData>> Controller<AppData, W> for EnterPrints {
  #[instrument(skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut AppData, env: &Env) {
    use druid::{KeyEvent,KbKey};

    let span = span!(Level::INFO, "EnterPrints");
    let _entered = span.enter();
    //event!(Level::DEBUG, "{:?}/{:?}", event, data);

    match (event, data.selected.clone()) {
      (Event::KeyUp(KeyEvent{key: KbKey::Enter, ..}), Some(ref val)) =>  println!("{}", val),
      _ => ()
    }
    child.event(ctx, event, data, env);
  }
}
