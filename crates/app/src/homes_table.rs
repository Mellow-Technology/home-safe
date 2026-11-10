use std::cmp::Ordering;
use std::rc::Rc;

use gpui_kit::component::menu::{PopupMenu, PopupMenuItem};
use gpui_kit::component::table::{Column, ColumnSort, TableDelegate, TableState};
use gpui_kit::component::{ActiveTheme as _, h_flex};
use gpui_kit::*;
use home_safe_core::Home;

use crate::format;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Col {
    Photo,
    Address,
    City,
    Price,
    Beds,
    Baths,
    Sqft,
    PricePerSqft,
    Year,
    Type,
    Status,
    Zestimate,
    Hoa,
    Saved,
}

const COLUMNS: &[Col] = &[
    Col::Photo,
    Col::Address,
    Col::City,
    Col::Price,
    Col::Beds,
    Col::Baths,
    Col::Sqft,
    Col::PricePerSqft,
    Col::Year,
    Col::Type,
    Col::Status,
    Col::Zestimate,
    Col::Hoa,
    Col::Saved,
];

impl Col {
    fn column(self) -> Column {
        let (key, name, width) = match self {
            Col::Photo => ("photo", "", 64.),
            Col::Address => ("address", "Address", 240.),
            Col::City => ("city", "City", 150.),
            Col::Price => ("price", "Price", 110.),
            Col::Beds => ("beds", "Beds", 60.),
            Col::Baths => ("baths", "Baths", 60.),
            Col::Sqft => ("sqft", "Sq Ft", 80.),
            Col::PricePerSqft => ("ppsf", "$/Sq Ft", 80.),
            Col::Year => ("year", "Built", 64.),
            Col::Type => ("type", "Type", 120.),
            Col::Status => ("status", "Status", 110.),
            Col::Zestimate => ("zestimate", "Zestimate", 110.),
            Col::Hoa => ("hoa", "HOA/mo", 80.),
            Col::Saved => ("saved", "Saved", 100.),
        };
        let col = Column::new(key, name).width(px(width));
        match self {
            Col::Photo => col.p_0().resizable(false),
            Col::Price
            | Col::Beds
            | Col::Baths
            | Col::Sqft
            | Col::PricePerSqft
            | Col::Year
            | Col::Zestimate
            | Col::Hoa => col.text_right().sortable(),
            _ => col.sortable(),
        }
    }

    fn text(self, h: &Home) -> String {
        match self {
            Col::Photo => String::new(),
            Col::Address => h.address.clone().unwrap_or_else(|| format!("zpid {}", h.source_id)),
            Col::City => [h.city.as_deref(), h.state.as_deref()]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", "),
            Col::Price => format::money(h.price),
            Col::Beds => format::decimal(h.beds),
            Col::Baths => format::decimal(h.baths),
            Col::Sqft => format::number(h.sqft),
            Col::PricePerSqft => format::money(h.price_per_sqft()),
            Col::Year => h.year_built.map(|y| y.to_string()).unwrap_or_default(),
            Col::Type => format::humanize(h.home_type.as_deref()),
            Col::Status => format::humanize(h.status.as_deref()),
            Col::Zestimate => format::money(h.zestimate),
            Col::Hoa => format::money(h.hoa_monthly),
            Col::Saved => format::date(&h.created_at),
        }
    }

    fn compare(self, a: &Home, b: &Home) -> Ordering {
        fn opt<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Ordering {
            // Missing values sort before present ones.
            a.partial_cmp(&b).unwrap_or(Ordering::Equal)
        }
        match self {
            Col::Price => opt(a.price, b.price),
            Col::Beds => opt(a.beds, b.beds),
            Col::Baths => opt(a.baths, b.baths),
            Col::Sqft => opt(a.sqft, b.sqft),
            Col::PricePerSqft => opt(a.price_per_sqft(), b.price_per_sqft()),
            Col::Year => opt(a.year_built, b.year_built),
            Col::Zestimate => opt(a.zestimate, b.zestimate),
            Col::Hoa => opt(a.hoa_monthly, b.hoa_monthly),
            Col::Saved => a.created_at.cmp(&b.created_at),
            _ => self.text(a).to_lowercase().cmp(&self.text(b).to_lowercase()),
        }
    }
}

/// Table data source. Keeps every home plus the filtered, sorted view of
/// them that the table displays.
pub struct HomesTable {
    homes: Vec<Home>,
    rows: Vec<usize>,
    filter: String,
    sort: Option<(Col, ColumnSort)>,
    on_delete: Option<RowAction>,
}

/// Handler for a row's context menu item, given the row's home.
pub type RowAction = Rc<dyn Fn(Home, &mut Window, &mut App)>;

impl HomesTable {
    pub fn new() -> Self {
        Self {
            homes: Vec::new(),
            rows: Vec::new(),
            filter: String::new(),
            sort: None,
            on_delete: None,
        }
    }

    /// Adds a "Delete" item to each row's context menu.
    pub fn on_delete(mut self, handler: impl Fn(Home, &mut Window, &mut App) + 'static) -> Self {
        self.on_delete = Some(Rc::new(handler));
        self
    }

    pub fn set_homes(&mut self, homes: Vec<Home>) {
        self.homes = homes;
        self.rebuild();
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.trim().to_lowercase();
        self.rebuild();
    }

    pub fn total(&self) -> usize {
        self.homes.len()
    }

    pub fn home(&self, row_ix: usize) -> Option<&Home> {
        self.rows.get(row_ix).map(|&i| &self.homes[i])
    }

    /// Homes after filtering and sorting, in display order.
    pub fn visible(&self) -> impl Iterator<Item = &Home> {
        self.rows.iter().map(|&i| &self.homes[i])
    }

    /// Message to show when there are no rows to display.
    pub fn empty_message(&self) -> &'static str {
        if self.homes.is_empty() {
            "No homes yet. Open a listing on Zillow and click “Save in Home Safe”."
        } else {
            "No homes match your search."
        }
    }

    pub fn row_of(&self, home_id: i64) -> Option<usize> {
        self.rows.iter().position(|&i| self.homes[i].id == home_id)
    }

    fn rebuild(&mut self) {
        let filter = &self.filter;
        self.rows = (0..self.homes.len())
            .filter(|&i| {
                filter.is_empty() || {
                    let h = &self.homes[i];
                    let haystack = format!(
                        "{} {} {}",
                        h.full_address(),
                        h.home_type.as_deref().unwrap_or(""),
                        h.status.as_deref().unwrap_or("")
                    )
                    .to_lowercase();
                    filter.split_whitespace().all(|term| haystack.contains(term))
                }
            })
            .collect();
        if let Some((col, sort)) = self.sort {
            let homes = &self.homes;
            self.rows.sort_by(|&a, &b| {
                let ord = col.compare(&homes[a], &homes[b]);
                if sort == ColumnSort::Descending { ord.reverse() } else { ord }
            });
        }
    }
}

impl TableDelegate for HomesTable {
    fn columns_count(&self, _: &App) -> usize {
        COLUMNS.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        COLUMNS[col_ix].column()
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.sort = match sort {
            ColumnSort::Default => None,
            sort => Some((COLUMNS[col_ix], sort)),
        };
        self.rebuild();
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(home) = self.home(row_ix) else {
            return div().into_any_element();
        };
        let col = COLUMNS[col_ix];
        if col == Col::Photo {
            let cell = h_flex().size_full().items_center().justify_center();
            return match &home.cover_image {
                Some(path) => cell
                    .child(
                        img(std::path::PathBuf::from(path))
                            .w(px(48.))
                            .h(px(28.))
                            .rounded(px(3.))
                            .object_fit(ObjectFit::Cover),
                    )
                    .into_any_element(),
                None => cell
                    .child(div().w(px(48.)).h(px(28.)).rounded(px(3.)).bg(cx.theme().muted))
                    .into_any_element(),
            };
        }
        div().truncate().child(col.text(home)).into_any_element()
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        let (Some(home), Some(on_delete)) = (self.home(row_ix).cloned(), self.on_delete.clone())
        else {
            return menu;
        };
        menu.item(
            PopupMenuItem::new("Delete…")
                .icon(gpui_kit::assets::IconName::Trash)
                .on_click(move |_, window, cx| on_delete(home.clone(), window, cx)),
        )
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _: &App) -> Option<SharedString> {
        self.home(row_ix).map(|h| COLUMNS[col_ix].text(h).into())
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
        self.home(row_ix).map(|h| COLUMNS[col_ix].text(h)).unwrap_or_default()
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let message = self.empty_message();
        h_flex()
            .size_full()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child(message)
    }
}
