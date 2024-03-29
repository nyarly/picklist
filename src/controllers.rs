use tracing::{debug, instrument};
use druid::widget::Controller;
use druid::{
  Data,
  Widget,
  WidgetId,
  EventCtx,
  Event,
  LifeCycle,
  LifeCycleCtx,
  Env,
  Selector,
};

pub struct TakeFocus;

impl<T, W: Widget<T>> Controller<T, W> for TakeFocus {
  #[instrument(name="TakeFocus", skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {

    if let Event::WindowConnected = event {
      ctx.request_focus();
    }
    child.event(ctx, event, data, env)
  }
}

pub struct SendFocus {
  delegate: WidgetId
}

impl SendFocus {
  pub fn new(to: WidgetId) -> Self {
    SendFocus{delegate: to}
  }
}

const GET_FOCUS: Selector = Selector::new("picklist.get_focus");

impl <D: Data, W: Widget<D>> Controller<D, W> for SendFocus {
  #[instrument(name="SendFocus",skip(self, child, ctx, event, data, env))]
  fn lifecycle(&mut self, child: &mut W, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &D, env: &Env) {
    match event {
      LifeCycle::FocusChanged(true) => ctx.submit_command(GET_FOCUS.to(self.delegate)),
      _ => ()
    }
    child.lifecycle(ctx, event, data, env);
  }
}


pub struct GetFocus;

impl <D: Data, W: Widget<D>> Controller<D, W> for GetFocus {
  #[instrument(name="GetFocus",skip(self, child, ctx, event, data, env))]
  fn event(&mut self, child: &mut W, ctx: &mut EventCtx, event: &Event, data: &mut D, env: &Env) {
    match event {
      Event::Command(cmd) if cmd.is(GET_FOCUS) => ctx.request_focus(),
      _ => ()
    }
    child.event(ctx, event, data, env);
  }
}
