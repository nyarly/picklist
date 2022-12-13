use druid::widget::{Flex, TextBox, List, Label, Controller, Scope, ViewSwitcher, Scroll};
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


type Filter = Rc<String>;
type PickItem = Rc<String>;
type PickData = (Vector<PickItem>, Option<PickItem>);

#[derive(Clone, Data, Lens, Debug)]
struct FilteredSelectionState {
  filter: String,
  sel_index: Option<usize>,
  list: Vector<Rc<String>>,
  selection: Option<Rc<String>>
}

impl FilteredSelectionState {
  pub fn new((list, selection): PickData) -> Self {
    FilteredSelectionState {
      filter: "".into(),
      sel_index: None,
      list,
      selection
    }

  }
}

fn main() -> Result<(), PlatformError> {
  let list_widget = picklist( || {
    ViewSwitcher::new(
      |data: &(Option<Rc<String>>, Rc<String>), _env: &_|
        if let (Some(sel), item) = data { sel == item } else { false },
      |selected: &bool, _data: &(Option<Rc<String>>, Rc<String>), _env: &_| {
        let label = Label::new(|data: &(Option<Rc<String>>, Rc<String>), _env: &_| {
          let (_sel, item) = data;
          (**item).clone()
        });

        if *selected { label.expand_width().border(Color::GREEN, 2.0).boxed() }
        else { label.expand_width().border(Color::BLUE, 2.0).boxed() }
      }
    )
  })
    .lens(( AppData::list, AppData::selected))
    .controller(DoneKeys{})
    .controller(EnterPrints{});

  let main_window = WindowDesc::new(list_widget);
  let data = AppData{
    selected: None,
    list: vector!(
      Rc::new("foobar".into()),
      Rc::new("foobaz".into()),
      Rc::new("foofarah".into())
    )
  };

  AppLauncher::with_window(main_window)
    //.log_to_console() // XXX if env DEBUG=true
    .launch(data)
}

// TODO:
// 1. Nicer layout - nicer "selected" appearance
// 2. Mouse events - click to select;
// 3. ? double click to select & exit...
// 4. Build for Linux/Mac/Win
// Reuse!
// 5. Focus Picklist => focus FilterText (and v/v)
// 6. Abstract off Strings - ideally be able to use e.g. a name or whatever in a struct

fn picklist<W: Widget<(Option<PickItem>, Filter)> + 'static>(wf: impl Fn() -> W + 'static) -> impl Widget<PickData> {
  let filter = TextBox::new()
    .expand_width()
    .lens(lens!((FilteredSelectionState, _), 0).then(FilteredSelectionState::filter))
    .controller(TakeFocus{});

  let list = List::new(wf)
  .lens((
      lens!((FilteredSelectionState, _), 0).then(FilteredSelectionState::selection),
      lens!((_, Vector<Rc<String>>), 1)
    ));


  let flex = Flex::column()
    .with_child(filter)
    .with_child(Scroll::new(list).vertical())
    .controller(MoveSelection{})
    .lens((
      lens::Identity{},
      (
        FilteredSelectionState::list,
        FilteredSelectionState::filter
      ).then(FilteredLens{})
    ));

  Scope::from_lens(
    FilteredSelectionState::new,
    (FilteredSelectionState::list, FilteredSelectionState::selection),
    flex
  )
}

struct MoveSelection;


type MoveSelData = (FilteredSelectionState, Vector<Rc<String>>);

impl<W: Widget<MoveSelData>> Controller<MoveSelData, W> for MoveSelection {
  #[instrument(skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut MoveSelData, env: &Env) {
    use druid::{KeyEvent,KbKey};
    //event!(Level::DEBUG, "{:?}/{:?}", event, data);
    //
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
      fss.selection = list.get(idx).cloned();
    }

    child.event(ctx, event, data, env);
  }
}

struct TakeFocus;

impl<T, W: Widget<T>> Controller<T, W> for TakeFocus {
  #[instrument(skip(self, child, ctx, event, data, env))]
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

struct DoneKeys;

impl<T, W: Widget<T>> Controller<T, W> for DoneKeys {
  #[instrument(skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
    use druid::{KeyEvent,KbKey,Application};

    let span = span!(Level::INFO, "DoneKeys");
    let _entered = span.enter();
    //event!(Level::DEBUG, "{:?}", event);

    child.event(ctx, event, data, env);
    match event {
      Event::KeyUp(KeyEvent{key: KbKey::Enter, ..}) => Application::global().quit(),
      Event::KeyUp(KeyEvent{key: KbKey::Escape, ..}) => Application::global().quit(),
      _ => ()
    }
  }
}
