pub mod filtered_lens;
pub mod controllers;

use crate::filtered_lens::{FuzzyMatchable,FilteredLens};
use crate::controllers::{SendFocus,GetFocus};

use druid::widget::{Flex, TextBox, List, Controller, Scope, Scroll};
use druid::{
  Data,
  Widget,
  WidgetId,
  WidgetExt,
  LensExt,
  Lens,
  EventCtx,
  Event,
  Env,
  lens
};
use druid::im::Vector;
use tracing::{instrument,debug};

type PickData<Sel> = (Option<Sel>, Vector<Sel>);

#[derive(Clone, Data, Lens, Debug)]
struct FilteredSelectionState<Sel: Clone> {
  filter: String,
  sel_index: Option<usize>,
  list: Vector<Sel>,
  selection: Option<Sel>
}

impl<Sel: Clone> FilteredSelectionState<Sel> {
  pub fn new((selection, list): PickData<Sel>) -> Self {
    FilteredSelectionState {
      filter: "".into(),
      sel_index: None,
      list,
      selection
    }
  }
}

pub fn picklist<Sel, W: Widget<(Option<Sel>, Sel)> + 'static>(wf: impl Fn() -> W + 'static) -> impl Widget<PickData<Sel>>
where Sel: Data+Default+FuzzyMatchable
{
  let filter_id = WidgetId::next();
  let filter = TextBox::new()
    .expand_width()
    .lens(lens!((FilteredSelectionState<Sel>, _), 0).then(FilteredSelectionState::filter))
    .with_id(filter_id)
    .controller(GetFocus{});

  let list = List::new(wf)
    .lens((
      lens!((FilteredSelectionState<Sel>, _), 0).then(FilteredSelectionState::selection),
      lens!((_, Vector<_>), 1)
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
    (FilteredSelectionState::selection, FilteredSelectionState::list),
    flex
  ).controller(SendFocus::new(filter_id))
}

trait Selectable<Sel> {
  fn select(&self, idx: usize) -> Option<Sel>;

  fn len(&self) -> usize;
}

impl<Sel: Clone> Selectable<Sel> for Vector<Sel> {
  fn select(&self, idx: usize) -> Option<Sel> {
    self.get(idx).cloned()
  }

  fn len(&self) -> usize {
    self.len()
  }
}

type MoveSelData<Sel> = (FilteredSelectionState<Sel>, Vector<Sel>);

struct MoveSelection;

impl<Sel: Clone, W: Widget<MoveSelData<Sel>>> Controller<MoveSelData<Sel>, W> for MoveSelection {
  #[instrument(skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut MoveSelData<Sel>, env: &Env) {
    use druid::{KeyEvent,KbKey};
    debug!("{:?}", event);
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
      fss.selection = list.select(idx);
    }

    child.event(ctx, event, data, env);
  }
}
