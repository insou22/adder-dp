mod app;
pub(crate) mod future;

use app::App;

fn main() {
    yew::start_app::<App>();
}
