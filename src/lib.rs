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
  UpdateCtx,
  EventCtx,
  Event,
  Env,
  Selector,
  lens
};
use druid::text::RichText;
use druid::im::Vector;
use tracing::{instrument,debug,info};

type PickData<Sel> = (Option<Sel>, Vector<Sel>);
type PickItem<Sel> = (Option<Sel>, (Sel, RichText));

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

pub fn picklist<Sel, W>(wf: impl Fn() -> W + 'static) -> impl Widget<PickData<Sel>>
where
  Sel: Data+Default+FuzzyMatchable+PartialEq,
  W: Widget<(Option<Sel>, (Sel, RichText))> + 'static
{
  let filter_id = WidgetId::next();
  let filter = TextBox::new()
    .expand_width()
    .lens(lens!((FilteredSelectionState<Sel>, _), 0).then(FilteredSelectionState::filter))
    .with_id(filter_id)
    .controller(GetFocus{});

  let list = List::new(move || wf().controller(ScrollSelected))
    .lens((
      lens!((FilteredSelectionState<Sel>, _), 0).then(FilteredSelectionState::selection),
      lens!((_, Vector<(Sel, RichText)>), 1)
    ));

  let sel_id = WidgetId::next();

  let flex = Flex::column()
    .with_child(filter)
    .with_flex_child(Scroll::new(list).vertical(), 1.0)
    .controller(MoveSelection{
      me: sel_id
    })
    .with_id(sel_id)
    .lens((
      lens::Identity{},
      (
        FilteredSelectionState::list,
        FilteredSelectionState::filter
      ).then::<_, Vector<(Sel, RichText)>>(FilteredLens::default())
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

struct ScrollSelected;

impl<Sel: Clone + PartialEq, W: Widget<PickItem<Sel>>> Controller<PickItem<Sel>, W> for ScrollSelected {
  #[instrument(name="ScrollSelected", skip(self, child, ctx, old, data, env))]
  fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old: &PickItem<Sel>, data: &PickItem<Sel>, env: &Env) {
    if let (Some(sel), (item, _)) = data {
      if sel == item {
        ctx.scroll_to_view()
      }
    }
    child.update(ctx, old, data, env)
  }
}

type MoveSelData<Sel> = (FilteredSelectionState<Sel>, Vector<(Sel, RichText)>);

struct MoveSelection{
  me: WidgetId
}

pub const INDEX_CHANGED: Selector = Selector::new("picklist.index_changed");
pub const SELECTION_CHANGED: Selector = Selector::new("picklist.selection_changed");

impl<Sel: Clone + PartialEq, W: Widget<MoveSelData<Sel>>> Controller<MoveSelData<Sel>, W> for MoveSelection {
  #[instrument(name="MoveSelection", skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut MoveSelData<Sel>, env: &Env) {

    child.event(ctx, event, data, env);

    use druid::{KeyEvent,KbKey};
    let (fss, list) = data;

    match event {
      Event::KeyDown(KeyEvent{key, ..}) =>
        match (key, fss.sel_index.clone()) {
          (KbKey::ArrowUp, None) =>  {
            debug!("Up/None");
            fss.sel_index = Some(list.len() - 1);
            ctx.submit_command(INDEX_CHANGED.to(self.me))
          },
          (KbKey::ArrowUp, Some(ref index)) => {
            debug!("Up/Some");
            fss.sel_index = index.checked_sub(1);
            ctx.submit_command(INDEX_CHANGED.to(self.me))
          },
          (KbKey::ArrowDown, None) => {
            debug!("Down/None");
            fss.sel_index = Some(0);
            ctx.submit_command(INDEX_CHANGED.to(self.me))
          },
          (KbKey::ArrowDown, Some(ref index)) => {
            debug!("Down/Some");
            fss.sel_index = index.checked_add(1);
            ctx.submit_command(INDEX_CHANGED.to(self.me))
          },
          _ => ()
        },
      Event::Command(cmd) => {
        if cmd.get(SELECTION_CHANGED).is_some() {
          if let Some(idx) = fss.sel_index {
            if fss.selection == list.select(idx).map(|(s,_)| s) {
              return
            }
          }
          match &fss.selection {
            Some(sel) => fss.sel_index = list.iter().position(|(s, _)| sel == s),
            None => fss.sel_index = None
        }

        }
        if cmd.get(INDEX_CHANGED).is_some() {
          match fss.sel_index {
            Some(idx) => fss.selection = list.select(idx).map(|(s,_)| s),
            None => fss.selection = None
          }
        }
      },
      _ => ()
    }

    if let Some(idx) = fss.sel_index {
      if idx >= list.len() {
        fss.sel_index = None
      }
    }

    if let Some(idx) = fss.sel_index {
      fss.selection = list.select(idx).map(|(s, _)| s);
    }
  }

  #[instrument(name="MoveSelection", skip(self, child, ctx, old_data, data, env))]
  fn update(&mut self, child: &mut W, ctx: &mut UpdateCtx, old_data: &MoveSelData<Sel>, data: &MoveSelData<Sel>, env: &Env) {
    let (old, _old_list) = old_data;
    let (new, _new_list) = data;
    debug!("data update");
    match (&old.sel_index, &new.sel_index, &old.selection, &new.selection) {
      (&None, &None, &None, &None) => (),
      (&Some(a), &Some(b), &None, &None) if a == b => (),
      (&None, &None, &Some(ref c), &Some(ref d)) if c == d => {
        debug!("selection not changed")
      },
      (&Some(a),&Some(b),&Some(ref c),&Some(ref d)) if a == b && c == d => (),

      (_, _,&Some(ref c),&Some(ref d)) if c == d => {
          debug!("index changed");
          ctx.submit_command(INDEX_CHANGED.to(self.me))
        },
      (_, _ , &None, &None) => {
          debug!("index changed");
          ctx.submit_command(INDEX_CHANGED.to(self.me))
        },

      (_, _, &Some(_), &Some(_)) |
        (_, _, &Some(_), &None) |
        (_, _, &None, &Some(_)) => {
          debug!("selection changed");
          ctx.submit_command(SELECTION_CHANGED.to(self.me))
        },
    }
    child.update(ctx, old_data, data, env);
  }

}
