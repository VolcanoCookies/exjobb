mod edge;
mod node;

use clap::Args;
pub use edge::EdgeColor;
pub use node::NodeColor;

use crate::output::DrawOptions;

#[derive(Debug, Clone, PartialEq, Args)]
#[group(required = false, multiple = true)]
pub struct LineStyle {
    #[clap(long, default_value = "1.0")]
    pub edge_width: f64,
    #[clap(long, default_value = "none")]
    pub edge_cap: Option<String>,
    #[clap(long, default_value = "none")]
    pub edge_join: Option<String>,
    #[clap(long, default_value = "none")]
    pub edge_dash: Option<String>,
}

impl LineStyle {
    pub fn to_draw<T: Into<String>>(&self, color: T) -> DrawOptions {
        let mut opts = DrawOptions {
            color: color.into(),
            stroke: self.edge_width as f32,
            ..Default::default()
        };

        if let Some(edge_cap) = &self.edge_cap {
            opts.stroke_linecap = edge_cap.clone();
        }
        if let Some(edge_join) = &self.edge_join {
            opts.stroke_linejoin = edge_join.clone();
        }
        if let Some(edge_dash) = &self.edge_dash {
            opts.stroke_dasharray = edge_dash.clone();
        }

        opts
    }
}
