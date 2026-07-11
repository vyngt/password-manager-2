use crate::primitives::text_prop::TextProp;
use crate::primitives::tokens::{Align, BadgeVariant, SortDirection};
use leptos::prelude::*;
use std::sync::Arc;

/// How a column's body cells should be rendered and styled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Text,
    Badge,
    Date,
    Mono,
    Action,
    Custom,
}

/// Width policy for a column — drives the `<col>` element's inline style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnWidth {
    Fixed(u32),
    Flexible,
    MinMax(u32, u32),
}

/// The value returned by a column's `cell` function for one row.
/// Which variant is valid depends on the column's `col_type`.
pub enum CellValue {
    Text(String),
    Badge {
        label: String,
        variant: BadgeVariant,
    },
    View(AnyView),
}

/// Boxed cell renderer used by `ColumnDef::cell`.
pub type CellFn<T> = Arc<dyn Fn(&T) -> CellValue + Send + Sync>;

/// Boxed string formatter used for `row_key` and `row_select_label`.
pub type StringFn<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;

/// Wrap a closure as a `CellFn<T>`. Use at column-def construction sites to
/// avoid writing the `Arc<dyn Fn...>` unsize cast by hand.
pub fn cell_fn<T, F>(f: F) -> CellFn<T>
where
    F: Fn(&T) -> CellValue + Send + Sync + 'static,
    T: 'static,
{
    Arc::new(f)
}

/// Wrap a closure as a `StringFn<T>`. Use for `row_key` and `row_select_label`.
pub fn string_fn<T, F>(f: F) -> StringFn<T>
where
    F: Fn(&T) -> String + Send + Sync + 'static,
    T: 'static,
{
    Arc::new(f)
}

/// One column's definition. Generic over the row type `T`.
pub struct ColumnDef<T: 'static> {
    pub id: &'static str,
    /// Header label. `TextProp` (not `&'static str`) so headers relocalize on a
    /// language switch without rebuilding the `columns` Vec.
    pub header: TextProp,
    pub col_type: ColumnType,
    pub sortable: bool,
    pub width: ColumnWidth,
    pub align: Align,
    pub cell: CellFn<T>,
}

impl<T: 'static> Clone for ColumnDef<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            header: self.header,
            col_type: self.col_type,
            sortable: self.sortable,
            width: self.width,
            align: self.align,
            cell: Arc::clone(&self.cell),
        }
    }
}

/// Controlled sort state — owned by the consumer, rendered by `DataTable`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortState {
    pub column_id: String,
    pub direction: SortDirection,
}
