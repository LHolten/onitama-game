use dominator::Dom;

pub trait Render {
    fn render(&self) -> Dom;
}

pub trait RenderOpt {
    fn render_opt(&self) -> Option<Dom>;
}

impl<T: Render> Render for Option<T> {
    fn render(&self) -> Dom {
        RenderOpt::render_opt(self).unwrap_or_else(Dom::empty)
    }
}

impl<T: Render> RenderOpt for Option<T> {
    fn render_opt(&self) -> Option<Dom> {
        self.as_ref().map(Render::render)
    }
}
