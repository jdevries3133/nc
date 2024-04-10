use super::models;
use crate::{
    components::{Chevron, ChevronVariant, Component, DeleteButton},
    models::{Prop, Value, ValueType},
    routes::Route,
};
use ammonia::clean;

pub struct FilterIcon;
impl Component for FilterIcon {
    fn render(&self) -> String {
        r##"
        <div id="filter-icon" class="rounded">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M12 3c2.755 0 5.455.232 8.083.678.533.09.917.556.917 1.096v1.044a2.25 2.25 0 01-.659 1.591l-5.432 5.432a2.25 2.25 0 00-.659 1.591v2.927a2.25 2.25 0 01-1.244 2.013L9.75 21v-6.568a2.25 2.25 0 00-.659-1.591L3.659 7.409A2.25 2.25 0 013 5.818V4.774c0-.54.384-1.006.917-1.096A48.32 48.32 0 0112 3z" />
            </svg>
        </div>
        <script>
            (() => {{
            const iconElement = document.querySelector("#filter-icon");
            iconElement.addEventListener('click', function () {{
                if ([...iconElement.classList].includes('text-black')) {{
                    iconElement.classList.remove('text-black');
                    iconElement.classList.remove('bg-yellow-100');
                    htmx.trigger('body', 'toggle-filter-toolbar');
                }} else {{
                    iconElement.classList.add('text-black');
                    iconElement.classList.add('bg-yellow-100');
                    htmx.trigger('body', 'toggle-filter-toolbar');
                }}
            }});
            }})()
        </script>
        "##.into()
    }
}

pub struct FilterToolbarPlaceholder {
    pub collection_id: i32,
}
impl Component for FilterToolbarPlaceholder {
    fn render(&self) -> String {
        let show_toolbar =
            Route::CollectionShowFilterToolbar(Some(self.collection_id));
        format!(
            r#"
            <div
                hx-get="{show_toolbar}"
                hx-trigger="toggle-filter-toolbar from:body"
                ></div>
            "#
        )
    }
}

pub struct FilterToolbar<'a> {
    pub filters: Vec<models::Filter>,
    pub collection_id: i32,
    pub get_prop_name: &'a dyn Fn(i32) -> &'a str,
}
impl Component for FilterToolbar<'_> {
    fn render(&self) -> String {
        let collection_id = self.collection_id;
        let rendered_filters =
            self.filters.iter().fold(String::new(), |mut acc, f| {
                let prop_name = (self.get_prop_name)(f.prop_id);
                acc.push_str(
                    &FilterChip {
                        filter: f,
                        prop_name,
                    }
                    .render(),
                );
                acc
            });
        let hide_toolbar =
            Route::CollectionHideSortToolbar(Some(collection_id));
        let add_filter = Route::CollectionAddFilterButton(Some(collection_id));
        format!(
            r#"
            <div
                hx-get="{hide_toolbar}"
                hx-trigger="toggle-filter-toolbar from:body"
                class="flex flex-row gap-2 mt-3 mb-4"
            >
            <div hx-trigger="load" hx-get="{add_filter}"></div>
                {rendered_filters}
            </div>
            "#
        )
    }
}

const FILTER_CONTAINER_STYLE: &str = "max-w-sm text-sm border border-slate-600 bg-gradient-to-tr from-blue-100 to-fuchsia-100 dark:bg-gradient-to-tr dark:from-fuchsia-800 dark:to-violet-700 rounded p-2 flex flex-row gap-2 items-center justify-center";

pub struct FilterChip<'a> {
    pub filter: &'a models::Filter,
    pub prop_name: &'a str,
}
impl Component for FilterChip<'_> {
    fn render(&self) -> String {
        let href = match &self.filter.value {
            models::FilterValue::Single(val) => self
                .filter
                .r#type
                .get_form_route(self.filter.id, ValueType::of_value(val)),
            models::FilterValue::Range(v1, _) => self
                .filter
                .r#type
                .get_form_route(self.filter.id, ValueType::of_value(v1)),
        };
        let subject = clean(self.prop_name);
        let operator_text = clean(self.filter.r#type.get_display_name());
        let range_arrow_icon = ArrowLeftRight {}.render();
        let rendered_value = match self.filter.r#type {
            models::FilterType::IsEmpty => "".into(),
            _ => match &self.filter.value {
                models::FilterValue::Single(val) => match val {
                    Value::Int(val) => format!("{val}"),
                    Value::Bool(val) => format!("{val}"),
                    Value::Date(val) => format!("{val}"),
                    Value::Float(val) => format!("{val}"),
                },
                models::FilterValue::Range(v1, v2) => match (v1, v2) {
                    (Value::Int(start), Value::Int(end)) => {
                        format!("{start} {range_arrow_icon} {end}")
                    }
                    (Value::Float(start), Value::Float(end)) => {
                        format!("{start} {range_arrow_icon} {end}")
                    }
                    (Value::Date(start), Value::Date(end)) => {
                        format!("{start} {range_arrow_icon} {end}")
                    }
                    (v1, v2) => {
                        panic!("{v1:?} and {v2:?} are different value types for ranged filter (component render)");
                    }
                },
            },
        };
        let chevron = Chevron {
            variant: ChevronVariant::Closed,
        }
        .render();
        let delete_btn = DeleteButton {
            delete_href: &href.as_string(),
            hx_target: Some("closest div"),
        }
        .render();
        format!(
            r#"
            <div
                hx-get="{href}"
                class="{FILTER_CONTAINER_STYLE} h-10 cursor-pointer">
                {chevron}
                <span class="text-xs">{subject}</span>
                <span class="text-[10px] bg-slate-100 bg-opacity-40 rounded text-black p-1 shadow italic whitespace-nowrap">{operator_text}</span>
                {rendered_value}
                {delete_btn}
            </div>
            "#
        )
    }
}

struct ArrowLeftRight;
impl Component for ArrowLeftRight {
    fn render(&self) -> String {
        r#"<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
          <path stroke-linecap="round" stroke-linejoin="round" d="M7.5 21L3 16.5m0 0L7.5 12M3 16.5h13.5m0-13.5L21 7.5m0 0L16.5 12M21 7.5H7.5" />
        </svg>"#.into()
    }
}

pub struct FilterForm<'a> {
    pub filter: &'a models::Filter,
    pub prop_name: &'a str,
}
impl Component for FilterForm<'_> {
    fn render(&self) -> String {
        match &self.filter.value {
            models::FilterValue::Single(value) => SingleFilterForm {
                id: self.filter.id,
                value,
                r#type: self.filter.r#type,
                prop_name: self.prop_name,
            }
            .render(),
            models::FilterValue::Range(start, end) => RangeFilterForm {
                id: self.filter.id,
                start,
                end,
                r#type: self.filter.r#type,
                prop_name: self.prop_name,
            }
            .render(),
        }
    }
}

pub struct SingleFilterForm<'a> {
    pub id: i32,
    pub value: &'a Value,
    pub r#type: models::FilterType,
    pub prop_name: &'a str,
}
impl Component for SingleFilterForm<'_> {
    fn render(&self) -> String {
        if let Value::Bool(_) = self.value {
            return BoolFilterForm {
                id: self.id,
                prop_name: self.prop_name,
            }
            .render();
        };
        let value_type = ValueType::of_value(self.value);
        let form_route = self.r#type.get_form_route(self.id, value_type);
        let chip_route = self.r#type.get_form_route(self.id, value_type);
        let filter_type_options = FilterTypeField {
            selected_type: self.r#type,
            value_type,
        }
        .render();
        let value_field = FilterValueField {
            value: self.value,
            label: "Value",
            name: "value",
        }
        .render();
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let prop_name = clean(self.prop_name);
        format!(
            r#"
            <form
                hx-post="{form_route}"
                class="{FILTER_CONTAINER_STYLE}"
            >
                <button 
                    class="self-start"
                    hx-get="{chip_route}"
                    hx-target="closest form"
                    >{chevron}</button>
                <div class="flex flex-col gap-2">
                    <h1 class="text-lg">{prop_name}</h1>
                    <div>
                        {filter_type_options}
                    </div>
                    <div>
                        {value_field}
                    </div>
                    <div>
                        <button class="dark:bg-slate-700 dark:text-white dark:hover:bg-slate-600 transition shadow hover:shadow-none rounded p-1 block">Save</button>
                    </div>
                </div>
            </form>
            "#
        )
    }
}

pub struct RangeFilterForm<'a> {
    pub id: i32,
    pub r#type: models::FilterType,
    pub start: &'a Value,
    pub end: &'a Value,
    pub prop_name: &'a str,
}
impl Component for RangeFilterForm<'_> {
    fn render(&self) -> String {
        if ValueType::of_value(self.start) != ValueType::of_value(self.end) {
            panic!("start and end are different value types");
        }
        let prop_name = self.prop_name;
        let chip_route = self
            .r#type
            .get_chip_route(self.id, ValueType::of_value(self.start));
        let form_route = self
            .r#type
            .get_form_route(self.id, ValueType::of_value(self.start));
        let type_field = FilterTypeField {
            selected_type: self.r#type,
            value_type: ValueType::of_value(self.start),
        }
        .render();
        let start_field = FilterValueField {
            value: self.start,
            label: "Start",
            name: "start",
        }
        .render();
        let end_filed = FilterValueField {
            value: self.end,
            label: "End",
            name: "end",
        }
        .render();
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        format!(
            r#"
            <form
                hx-post="{form_route}"
                class="{FILTER_CONTAINER_STYLE}"
            >
                <button 
                    hx-get="{chip_route}"
                    hx-target="closest form"
                    class="self-start"
                    >{chevron}</button>
                <div class="flex flex-col gap-2">
                    <h1 class="text-xl">{prop_name}</h1>
                    <div>{type_field}</div>
                    <div class="flex flex-col">
                        {start_field}
                    </div>
                    <div class="flex flex-col">
                        {end_filed}
                    </div>
                    <div>
                        <button class="dark:bg-slate-700 dark:text-white dark:hover:bg-slate-600 transition shadow hover:shadow-none rounded p-1 block">Save</button>
                    </div>
                </div>
            </form>
            "#
        )
    }
}

pub struct FilterTypeField {
    pub selected_type: models::FilterType,
    /// Value type will constrain the filter types that are available based
    /// on [models::FilterType::get_supported_filter_types]
    pub value_type: ValueType,
}
impl Component for FilterTypeField {
    fn render(&self) -> String {
        let options =
            models::FilterType::get_supported_filter_types(self.value_type);
        let html_options = options.iter().map(|option| {
            let int_repr = option.get_int_repr();
            let description = option.get_display_name();
            if option == &self.selected_type {
                format!(r#"<option selected value="{int_repr}">{description}</option>"#)
            } else {
                format!(r#"<option value="{int_repr}">{description}</option>"#)
            }
        }).collect::<Vec<String>>().join("\n");
        format!(
            r#"
            <label class="text-sm" for="type">Filter Type</label>
            <select
                id="type"
                name="type"
                class="dark:text-white text-sm dark:bg-slate-700 rounded"
                >{html_options}</select>
            "#
        )
    }
}

pub struct FilterValueField<'a> {
    pub value: &'a Value,
    pub label: &'a str,
    pub name: &'a str,
}
impl Component for FilterValueField<'_> {
    fn render(&self) -> String {
        let name = self.name;
        let label = self.label;
        match self.value {
            Value::Float(val) => {
                format!(
                    r#"
                    <label for="{name}">{label}</label>
                    <input
                        id="{name}"
                        name="{name}"
                        type="number"
                        step="0.01"
                        value="{val}" />
                    "#
                )
            }
            Value::Date(val) => {
                format!(
                    r#"
                    <label for="{name}">{label}</label>
                    <input
                        id="{name}"
                        name="{name}"
                        type="date"
                        value="{val}" />
                    "#
                )
            }
            Value::Bool(val) => {
                let checked = if *val { "checked" } else { "" };
                format!(
                    r#"
                    <label for="{name}">{label}</label>
                    <input
                        id="{name}"
                        name="{name}"
                        type="checkbox"
                        {checked} />
                    "#
                )
            }
            Value::Int(val) => {
                format!(
                    r#"
                    <label for="{name}">{label}</label>
                    <input
                        id="{name}"
                        name="{name}"
                        type="number"
                        value="{val}" />
                    "#
                )
            }
        }
    }
}

/// Booleans have a specialized form with nice buttons which is structurally
/// dissimilar compared to other data-types.
struct BoolFilterForm<'a> {
    id: i32,
    prop_name: &'a str,
}
impl Component for BoolFilterForm<'_> {
    fn render(&self) -> String {
        let filter_id = self.id;
        let container_id = format!("filter-{filter_id}-form");
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let prop_name = self.prop_name;
        let submit_url = Route::FilterBool(Some(filter_id));
        let chip_route = Route::FilterBoolChip(Some(filter_id));
        format!(
            r##"
            <div id="{container_id}" class="{FILTER_CONTAINER_STYLE} flex-col">
                <div class="flex flex-row">
                    <button 
                        hx-get="{chip_route}"
                        hx-target="#{container_id}""
                        >{chevron}</button>
                    <div class="flex flex-col">
                        <p class="text-xl">{prop_name}</p>
                        <p class="italic">update filter</p>
                    </div>
                </div>
                <div class="flex items-center justify-center">
                    <form
                        hx-target="#{container_id}"
                        hx-post="{submit_url}"
                        >
                            <input type="hidden" name="value" value="true" />
                            <button
                                class="bg-green-100 shadow hover:shadow-none hover:bg-green-200 dark:bg-green-700 dark:hover:bg-green-600 transition rounded-tl rounded-bl p-4"
                                >
                                True</button>
                    </form>
                    <form
                        hx-target="#{container_id}"
                        hx-post="{submit_url}"
                        >
                            <input type="hidden" name="value" value="is-empty" />
                            <button
                                class="bg-slate-100 shadow hover:shadow-none hover:bg-slate-200 dark:bg-slate-700 dark:hover:bg-slate-600 transition p-4 whitespace-nowrap"
                                >
                                Is Empty</button>
                    </form>
                    <form
                        hx-target="#{container_id}"
                        hx-post="{submit_url}"
                        >
                        <input type="hidden" name="value" value="false" />
                        <button
                            class="bg-red-100 shadow hover:shadow-none hover:bg-red-200 dark:bg-red-700 hover:dark:bg-red-600 transition rounded-tr rounded-br p-4"
                            >False</button>
                    </form>
                </div>
            </div>
            "##
        )
    }
}

pub struct ChoosePropForFilter<'a> {
    pub props: &'a Vec<&'a Prop>,
}
impl Component for ChoosePropForFilter<'_> {
    fn render(&self) -> String {
        let button_style = "p-2 w-full text-md rounded dark:bg-blue-700 dark:hover:bg-blue-600 shadow hover:shadow-none";
        let prop_buttons =
            self.props.iter().fold(String::new(), |mut acc, p| {
                let prop_id = p.id;
                let prop_name = clean(&p.name);
                let href = Route::PropNewFilterTypeSelect(Some(prop_id));
                let type_string = match p.type_id {
                    ValueType::Int => "integer",
                    ValueType::Bool => "checkbox",
                    ValueType::Float => "percent",
                    ValueType::Date => "date",
                };
                acc.push_str(&format!(
                    r#"
                    <button
                        hx-get="{href}"
                        hx-target="closest div"
                        class="{button_style}"
                        >{prop_name} ({type_string})</button>
                    "#
                ));
                acc
            });

        format!(
            r#"
            <div class="flex flex-col {FILTER_CONTAINER_STYLE}">
                {prop_buttons}
            </div>
            "#
        )
    }
}

pub struct AddFilterButton {
    pub collection_id: i32,
}
impl Component for AddFilterButton {
    fn render(&self) -> String {
        let collection_route =
            Route::CollectionChoosePropForFilter(Some(self.collection_id));
        format!(
            r#"
            <button
                class="
                    p-2
                    max-h-[45px]
                    whitespace-nowrap
                    rounded-lg
                    bg-blue-600
                    hover:bg-blue-500
                    shadow
                    hover:shadow-none
                    border-2
                    border-slate-600
                "
                hx-get="{collection_route}"
                >
                Add filter
            </button>
            "#
        )
    }
}

/// If the collection does not have capacity for any more filters, we will
/// render this component instead of the add filter button above. It is hidden
/// but it will receive events from Hx-Trigger headers when filters are
/// deleted, meaning that we've most likely gained capacity for a new filter
/// again.
pub struct AddFilterButtonPlaceholder {
    pub collection_id: i32,
}
impl Component for AddFilterButtonPlaceholder {
    fn render(&self) -> String {
        let route = Route::CollectionAddFilterButton(Some(self.collection_id));
        format!(
            r#"
            <div
                hx-get="{route}"
                hx-trigger="reload-add-filter-button"
            />
            "#
        )
    }
}

/// Kind of a hack to get the ending chunk of the URL and query parameters to
/// dynamically construct a URL for the appropriate new filter form.
fn get_slug(filter_type: models::FilterType, prop_type: ValueType) -> String {
    let filter_type_id = filter_type.get_int_repr();
    match filter_type {
        models::FilterType::Neq
        | models::FilterType::Lt
        | models::FilterType::Gt
        | models::FilterType::Eq
        | models::FilterType::IsEmpty => match prop_type {
            ValueType::Bool => {
                format!("new-bool-filter?type_id={filter_type_id}")
            }
            ValueType::Int => {
                format!("new-int-filter?type_id={filter_type_id}")
            }
            ValueType::Float => {
                format!("new-float-filter?type_id={filter_type_id}")
            }
            ValueType::Date => {
                format!("new-date-filter?type_id={filter_type_id}")
            }
        },
        models::FilterType::InRng | models::FilterType::NotInRng => {
            match prop_type {
                ValueType::Bool => {
                    panic!("in-rng and not-in-rng not supported for bool")
                }
                ValueType::Int => {
                    format!("new-int-rng-filter?type_id={filter_type_id}")
                }
                ValueType::Float => {
                    format!("new-float-rng-filter?type_id={filter_type_id}")
                }
                ValueType::Date => {
                    format!("new-date-rng-filter?type_id={filter_type_id}")
                }
            }
        }
    }
}

pub struct NewFilterTypeOptions<'a> {
    pub options: &'a Vec<models::FilterType>,
    pub prop_id: i32,
    pub prop_type: ValueType,
}
impl Component for NewFilterTypeOptions<'_> {
    fn render(&self) -> String {
        let button_style = "p-2 w-full text-md rounded dark:bg-blue-700 dark:hover:bg-blue-600 shadow hover:shadow-none";
        let prop_id = self.prop_id;
        let rendered_options =
            self.options.iter().fold(String::new(), |mut str, opt| {
                let opt_text = clean(opt.get_display_name());
                let type_slug = get_slug(*opt, self.prop_type);
                str.push_str(&format!(
                    r#"
                    <button
                        class="{button_style}"
                        hx-post="/prop/{prop_id}/{type_slug}"
                        hx-target="closest div"
                        >{opt_text}</button>
                    "#
                ));
                str
            });
        format!(
            r#"
            <div class="flex flex-col {FILTER_CONTAINER_STYLE}"
                >
                <p class="text-lg">Choose Filter Type</p>
                {rendered_options}
            </div>
            "#
        )
    }
}
