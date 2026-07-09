//! SignalKit demo app for Kindle.
//!
//! A single reactive screen: a counter label bound to a signal, +/- buttons, a
//! keyed `for_each` list with an "Add row" button, and an "Exit" button that
//! stops the event loop. On device it runs the real FBInk + touch loop; on the
//! host (no `fbink` feature) it renders one frame to a mock renderer so the
//! binary still builds and can be smoke-tested.

use signalkit::{
    for_each, hstack, Align, Axis, BuildCtx, Button, Color, Component, ExitHandle, IntoNode, Label,
    Node, Signal, Spacer,
};

#[derive(Clone)]
struct Row {
    id: u32,
    text: String,
}

struct DemoScreen {
    count: Signal<i32>,
    rows: Signal<Vec<Row>>,
    next_id: std::rc::Rc<std::cell::Cell<u32>>,
    exit: ExitHandle,
}

impl DemoScreen {
    fn new(exit: ExitHandle) -> Self {
        DemoScreen {
            count: Signal::new(0),
            rows: Signal::new(vec![
                Row { id: 1, text: "First row".into() },
                Row { id: 2, text: "Second row".into() },
            ]),
            next_id: std::rc::Rc::new(std::cell::Cell::new(3)),
            exit,
        }
    }
}

impl Component for DemoScreen {
    fn build(&mut self, ctx: &mut BuildCtx) -> Node {
        // Counter label bound to the count signal.
        let counter = Label::new("").size(3);
        {
            let counter = counter.clone();
            ctx.bind(&self.count, move |v| counter.set_text(format!("Count: {v}")));
        }

        let dec = {
            let count = self.count.clone();
            Button::new(" - ").size(3).on_tap(move || count.update(|v| v - 1))
        };
        let inc = {
            let count = self.count.clone();
            Button::new(" + ").size(3).on_tap(move || count.update(|v| v + 1))
        };
        let buttons = hstack(24, Align::Center, vec![dec.into_node(), inc.into_node()]);

        // Keyed list of rows.
        let list = for_each(
            &self.rows,
            |r| r.id,
            |r| Label::new(r.text.clone()).size(2).into_node(),
        );

        let add = {
            let rows = self.rows.clone();
            let next_id = self.next_id.clone();
            Button::new("Add row").size(2).on_tap(move || {
                let id = next_id.get();
                next_id.set(id + 1);
                rows.update(|current| {
                    let mut v = current.clone();
                    v.push(Row { id, text: format!("Row {id}") });
                    v
                });
            })
        };

        let exit_btn = {
            let exit = self.exit.clone();
            Button::new("Exit").size(2).on_tap(move || exit.request_exit())
        };

        // A padded, white-background frame wrapping the whole screen.
        Node::Stack {
            axis: Axis::Vertical,
            spacing: 20,
            padding: 24,
            align: Align::Center,
            bg: Some(Color::WHITE),
            children: vec![
                Label::new("SignalKit Demo").size(4).into_node(),
                counter.into_node(),
                buttons,
                Label::new("Rows:").size(2).into_node(),
                list,
                add.into_node(),
                Spacer::new().into_node(),
                exit_btn.into_node(),
            ],
        }
    }
}

fn root(exit: ExitHandle) -> Box<dyn Component> {
    Box::new(DemoScreen::new(exit))
}

#[cfg(feature = "fbink")]
fn main() -> std::io::Result<()> {
    use signalkit::render::fbink::FbinkRenderer;
    use signalkit::App;

    let exit = ExitHandle::new();
    let renderer = FbinkRenderer::open()?;
    let mut app = App::new(root(exit.clone()), renderer, exit);
    app.run()
}

#[cfg(not(feature = "fbink"))]
fn main() -> std::io::Result<()> {
    // Host smoke test: mount, render one frame to the mock renderer, report.
    use signalkit::geometry::Size;
    use signalkit::render::mock::MockRenderer;
    use signalkit::App;

    let exit = ExitHandle::new();
    let mut app = App::new(
        root(exit.clone()),
        MockRenderer::new(Size::new(600, 800)),
        exit,
    );
    app.render_frame()?;
    println!("signalkit-demo host stub: rendered one frame (no FBInk).");
    Ok(())
}
