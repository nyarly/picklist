use druid::widget::{Flex, TextBox, List, Label, Controller, Scope, ViewSwitcher};
use druid::{AppLauncher, Color, Data, PlatformError, Widget, WidgetExt, LensExt, WindowDesc, Lens, EventCtx, Event, Env, lens};
use druid::im::{vector,Vector};
use std::rc::Rc;
use fuzzy_matcher::skim::SkimMatcherV2;
use tracing::{span, event, Level, instrument};

#[derive(Clone, Data, Lens, Debug)]
struct AppData {
  list: Vector<Rc<String>>,
  selected: Option<Rc<String>>
}

struct FilteredLens { }

#[derive(Clone, Data, Debug)]
struct FilterMatch<T: Data>{
  text: T,
  index: usize,
  score: i64
}

trait FuzzyMatchable {
  fn match_against(&self) -> &str;
}

impl FuzzyMatchable for Rc<String> {
  fn match_against(&self) -> &str {
    let s = self.as_ref();
    &s
  }
}

impl FilteredLens {
  fn match_items<'a, L, V>(&self, list: &L, filter: &str) -> Vector<FilterMatch<V>>
  where L: Data+IntoIterator<Item = V>,
   V: Data+FuzzyMatchable
  {
    let matcher = SkimMatcherV2::default();
    // XXX clone, collect
    list.clone().into_iter().enumerate().filter_map(|(index, text)| {
      let t = text.match_against();
      matcher.fuzzy(t, &filter, false).map(|(score,_)| FilterMatch{text: text.clone(), index, score})
    }).collect()
  }
}

impl Lens<(Vector<Rc<String>>,String), Vector<Rc<String>>> for FilteredLens {
  fn with<V, F: FnOnce(&Vector<Rc<String>>) -> V>(&self, data: &(Vector<Rc<String>>, String), f: F) -> V {
    let (list, filter) = data;
    let matches = self.match_items(list, filter).iter().map(|fm| fm.text.clone()).collect();
    f(&matches)
  }

  fn with_mut<V, F: FnOnce(&mut Vector<Rc<String>>) -> V>(&self, data: &mut (Vector<Rc<String>>, String), f: F) -> V {
    let (list, filter) = data;
    let mut matches = self.match_items(list, filter).iter().map(|fm| fm.text.clone()).collect();

    f(&mut matches)
  }
}


fn main() -> Result<(), PlatformError> {
  let main_window = WindowDesc::new(ui_builder());
  let data = AppData{
    selected: None,
    list: vector!(
      Rc::new("foobar".into()),
      Rc::new("foobaz".into()),
      Rc::new("foofarah".into())
    )
  };

  AppLauncher::with_window(main_window)
    .log_to_console()
    .launch(data)
}

#[derive(Clone, Data, Lens, Debug)]
struct FilteredSelectionState {
  filter: String,
  sel_index: Option<usize>,
  app_data: AppData
}

impl FilteredSelectionState {
  pub fn new(app_data: AppData) -> Self {
    FilteredSelectionState {
      filter: "".into(),
      sel_index: None,
      app_data
    }

  }
}

fn ui_builder() -> impl Widget<AppData> {
  let filter = TextBox::new()
    .lens(lens!((FilteredSelectionState, _), 0).then(FilteredSelectionState::filter))
    .controller(TakeFocus{});

  let list = List::new(|| {
    ViewSwitcher::new(
      |data: &(Option<Rc<String>>, Rc<String>), _env: &_|
        if let (Some(sel), item) = data { sel == item } else { false },
      |selected: &bool, _data: &(Option<Rc<String>>, Rc<String>), _env: &_| {
        let label = Label::new(|data: &(Option<Rc<String>>, Rc<String>), _env: &_| {
          let (_sel, item) = data;
          (**item).clone()
        });

        if *selected { label.border(Color::GREEN, 2.0).boxed() }
        else { label.boxed() }
      }
    )
  })
  .lens((
      lens!((FilteredSelectionState, _), 0).then(FilteredSelectionState::app_data.then(AppData::selected)),
      lens!((_, Vector<Rc<String>>), 1)
    ));


  let flex = Flex::column()
    .with_child(filter)
    .with_child(list)
    .controller(MoveSelection{})
    .controller(DoneKeys{})
    .lens((
      lens::Identity{},
      (
        FilteredSelectionState::app_data.then(AppData::list),
        FilteredSelectionState::filter
      ).then(FilteredLens{})
    ));

  Scope::from_lens(
    FilteredSelectionState::new,
    FilteredSelectionState::app_data,
    flex
  ).controller(EnterPrints{})
}

struct MoveSelection;


type MoveSelData = (FilteredSelectionState, Vector<Rc<String>>);

impl<W: Widget<MoveSelData>> Controller<MoveSelData, W> for MoveSelection {
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut MoveSelData, env: &Env) {
    use druid::{KeyEvent,KbKey};
    let span = span!(Level::INFO, "MoveSelection");
    let _entered = span.enter();
    //event!(Level::DEBUG, "{:?}/{:?}", event, data);

    let (fss, list) = data;

    match (event, fss.sel_index.clone()) {
      (Event::KeyUp(KeyEvent{key: KbKey::ArrowUp, ..}), None) =>  fss.sel_index = Some(list.len() - 1),
      (Event::KeyUp(KeyEvent{key: KbKey::ArrowUp, ..}), Some(ref index)) => fss.sel_index = index.checked_sub(1),
      (Event::KeyUp(KeyEvent{key: KbKey::ArrowDown, ..}), None) => fss.sel_index = Some(0),
      (Event::KeyUp(KeyEvent{key: KbKey::ArrowDown, ..}), Some(ref index)) => fss.sel_index = index.checked_add(1),
      _ => ()
    }

    if let Some(idx) = fss.sel_index {
      if idx >= list.len() {
        fss.sel_index = None
      }
    }

    if let Some(idx) = fss.sel_index {
      fss.app_data.selected = list.get(idx).cloned();
    }

    child.event(ctx, event, data, env);
  }
}

struct TakeFocus;

impl<T, W: Widget<T>> Controller<T, W> for TakeFocus {
    fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
      let span = span!(Level::INFO, "TakeFocus");
      let _entered = span.enter();
      // event!(Level::DEBUG, "{:?}", event);

      if let Event::WindowConnected = event {
          ctx.request_focus();
      }
      child.event(ctx, event, data, env)
    }
}

struct EnterPrints;

impl<W: Widget<AppData>> Controller<AppData, W> for EnterPrints {
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

struct DoneKeys;

impl<T, W: Widget<T>> Controller<T, W> for DoneKeys {
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
    use druid::{KeyEvent,KbKey,Application};

    let span = span!(Level::INFO, "DoneKeys");
    let _entered = span.enter();
    // event!(Level::DEBUG, "{:?}/{:?}", event, data);

    child.event(ctx, event, data, env);
    match event {
      Event::KeyUp(KeyEvent{key: KbKey::Enter, ..}) => Application::global().quit(),
      Event::KeyUp(KeyEvent{key: KbKey::Escape, ..}) => Application::global().quit(),
      _ => ()
    }
  }
}
