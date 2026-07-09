//! Function pane sub-pages.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FunctionPage {
    #[default]
    Workspace,
    Connections,
}
