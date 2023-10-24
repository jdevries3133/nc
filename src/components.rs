// In many cases, we need to do a let binding to satisfy the borrow checker
// and for some reason, clippy identifies those as unnecessary. Maybe there
// are and clippy knows more than me, maybe not.
#![allow(clippy::let_and_return)]

use super::{models, routes::Route};
use ammonia::clean;
use std::fmt::{Display, Write};

#[cfg(feature = "live_reload")]
const LIVE_RELOAD_SCRIPT: &str = r#"<script>
    (async () => {
        while (true) {
            try {
                await fetch('/ping?poll_interval_secs=60');
            } catch (e) {
                console.log("hup from ping; let's live-reload");
                const el = document.createElement('p');
                el.innerText = "Reloading...";
                el.classList.add("bg-yellow-100");
                el.classList.add("p-2");
                el.classList.add("rounded");
                el.classList.add("w-full");
                el.classList.add("dark:text-black");
                document.body.insertBefore(el, document.body.firstChild);
                setInterval(async () => {
                    setTimeout(() => {
                        // At some point, a compiler error may be preventing
                        // the server from coming back
                        el.innerText = "Reload taking longer than usual; check for a compiler error";
                    }, 2000);
                    // Now the server is down, we'll fast-poll it (trying to
                    // get an immediate response), and reload the page when it
                    // comes back
                    try {
                        await fetch('/ping?poll_interval_secs=0');
                        window.location.reload()
                    } catch (e) {}
                }, 100);
                break;
            }
        }
    })();
</script>"#;

#[cfg(not(feature = "live_reload"))]
const LIVE_RELOAD_SCRIPT: &str = "";

pub trait Component {
    /// Render the component to a HTML string. By convention, the
    /// implementation should sanitize all string properties at render-time
    fn render(&self) -> String;
}

pub struct Page<'a> {
    pub title: &'a str,
    pub children: Box<dyn Component + 'a>,
}

impl Component for Page<'_> {
    fn render(&self) -> String {
        // note: we'll get a compiler error here until the tailwind build occurs.
        // Make sure you use `make build` in the Makefile to get both to happen
        // together
        let tailwind = include_str!("./tailwind.generated.css");
        let htmx = Route::Htmx;
        format!(
            r#"
            <html>
                <head>
                    <meta name="viewport" content="width=device-width, initial-scale=1.0"></meta>
                    <title>{title}</title>
                    <style>
                        {tailwind}
                    </style>
                    {LIVE_RELOAD_SCRIPT}
                </head>
                <body hx-boost="true" class="dark:bg-indigo-1000 dark:text-white mt-2 ml-2 sm:mt-8 sm:ml-8">
                    {body_html}
                    <script src="{htmx}"></script>
                    <script>
                        htmx.config.defaultSwapStyle = "outerHTML"
                    </script>
                </body>
            </html>
            "#,
            tailwind = tailwind,
            title = clean(self.title),
            body_html = self.children.render()
        )
    }
}

pub struct Home;
impl Component for Home {
    fn render(&self) -> String {
        let design_link = ExternalLink {
            children: Box::new(Text {
                inner_text: &"Click here to learn more about the design and vision for the project"
            }),
            href: "https://github.com/jdevries3133/nc/blob/main/DESIGN.md"
        }.render();
        let roadmap_link = ExternalLink {
            children: Box::new(Text {
                inner_text: &"Click here to learn about the project roadmap",
            }),
            href: "https://github.com/jdevries3133/nc#next-steps",
        }
        .render();
        let register_route = Route::Register;
        let login_route = Route::Login;
        format!(
            r#"
            <div class="prose bg-slate-200 rounded p-2">
                <h1>Notion Clone!</h1>
                <p>This is my simple Notion Clone, a work-in-progress.</p>
                <p>{design_link}</p>
                <p>{roadmap_link}</p>
                <a href="{register_route}">
                    <p>Click here to create an account use the app</p>
                </a>
                <a href="{login_route}">
                    <p>Click here to login.</p>
                </a>
            </div>
            "#
        )
    }
}

pub struct Collection {
    pub id: i32,
    pub name: String,
}
impl Component for Collection {
    fn render(&self) -> String {
        let id = self.id;
        let col_order = HoverIcon {
            children: Box::new(ColumnOrderIcon { collection_id: id }),
            tooltip_text: "Edit Column Order",
        }
        .render();
        let filter_icon = HoverIcon {
            children: Box::new(FilterIcon {}),
            tooltip_text: "View Filters",
        }
        .render();
        let filter_toolbar_placeholder =
            FilterToolbarPlaceholder { collection_id: id }.render();
        let sort_icon = HoverIcon {
            children: Box::new(SortIcon { collection_id: id }),
            tooltip_text: "Configure Sorting",
        }
        .render();
        let sort_toolbar_placeholder =
            SortToolbarPlaceholder { collection_id: id }.render();
        let new_page_route = Route::CollectionNewPageForm(Some(id));
        let list_page_route = Route::CollectionListPages(Some(id));
        let name = clean(&self.name);
        format!(
            r#"
                <h1 class="serif text-xl my-4">{name}</h1>
                <a class="link" href="{new_page_route}">Create Page</a>
                <div class="mt-2 flex">
                    {col_order} {filter_icon} {sort_icon}
                </div>
                {filter_toolbar_placeholder}
                {sort_toolbar_placeholder}
                <main hx-trigger="load" hx-get="{list_page_route}">Loading Pages...</main>
            "#
        )
    }
}

pub struct PageList<'a> {
    pub pages: &'a [models::Page],
    pub props: &'a [models::Prop],
    pub collection_id: i32,
}
impl Component for PageList<'_> {
    fn render(&self) -> String {
        let collection_id = self.collection_id;
        if self.pages.is_empty() {
            let list_page_route =
                Route::CollectionListPages(Some(collection_id));
            return format!(
                r#"
                <div
                    hx-get="{list_page_route}"
                    hx-trigger="reload-pages from:body"
                    >
                    <div>
                        <p>No pages matching filters are available</p>
                    </div>
                </div>
                "#
            );
        };
        let list = self.pages.iter().fold(String::new(), |mut str, page| {
                let page_route = Route::Page(Some(page.id));
                let title = clean(&page.title);
                let other_props = page
                    .props
                    .iter()
                    .map(|p| p.render())
                    .collect::<Vec<String>>()
                    .join("");
            let _ = write!(
                str,
                r#"
                    <div class="flex gap-2">
                        <a class="link" href="{page_route}">Edit</a>
                        <div class="max-w-[50vw] sm:max-w-xs truncate">{title}</div>
                    </div>
                    {other_props}
                "#,
            );
            str
        });
        // We're about to assume all pages are in the same collection... let's
        // enforce that invariant here at runtime just to be safe.
        let mut collection: Option<i32> = None;
        for page in self.pages {
            let cid = page.collection_id;
            if let Some(other_cid) = collection {
                if cid != other_cid {
                    panic!("cannot render mixed collection including {cid} and {other_cid}");
                }
                collection = Some(cid);
            }
        }
        let header = self.props.iter().fold(
            String::from(r#"<p class="text-center">Title</p>"#),
            |mut str, prop| {
                let prop_name = clean(&prop.name);
                str.push_str(&format!(
                    r#"<p class="text-center">{prop_name}</p>"#
                ));
                str
            },
        );
        // "+ 1" because we're accountign for the leftmost column containing
        // the "edit" button and the page title.
        let column_count = self.props.len() + 1;
        let list_page_route = Route::CollectionListPages(Some(collection_id));
        format!(
            r#"
            <div
                hx-get="{list_page_route}"
                hx-trigger="reload-pages from:body"
                class="mt-8 overflow-y-scroll grid gap-2"
                style="grid-template-columns: repeat({column_count}, auto);"
                >
                    {header}
                    {list}
            </div>
            "#
        )
    }
}

struct ColumnOrderIcon {
    collection_id: i32,
}
impl Component for ColumnOrderIcon {
    fn render(&self) -> String {
        let cid = self.collection_id;
        let prop_order_route = Route::CollectionChangePropOrder(Some(cid));
        format!(
            r#"
            <a href="{prop_order_route}">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 rotate-90">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 5.25h16.5m-16.5 4.5h16.5m-16.5 4.5h16.5m-16.5 4.5h16.5" />
                </svg>
            </a>
            "#
        )
    }
}

struct HoverIcon<'a> {
    pub children: Box<dyn Component>,
    pub tooltip_text: &'a str,
}
impl Component for HoverIcon<'_> {
    fn render(&self) -> String {
        let text = clean(self.tooltip_text);
        let children = self.children.render();
        format!(
            r#"
                <div data-tooltip="{text}" class="tooltip cursor-pointer">
                    {children}
                </div>
            "#
        )
    }
}

pub struct PageOverview<'a> {
    pub page: &'a models::Page,
}
impl Component for PageOverview<'_> {
    fn render(&self) -> String {
        let collection_route = Route::Collection(Some(self.page.collection_id));
        let back_button = format!(
            r#"
                <a class="block mb-2 link" href="{collection_route}">Back</a>
            "#,
        );
        [
            back_button,
            PageForm { page: self.page }.render(),
            ContentDisplay {
                page_id: self.page.id,
                content: self.page.content.as_ref(),
            }
            .render(),
        ]
        .join("\n")
    }
}

pub struct PageForm<'a> {
    pub page: &'a models::Page,
}
impl Component for PageForm<'_> {
    fn render(&self) -> String {
        let id = self.page.id;
        let title = &self.page.title;
        let collection_id = &self.page.collection_id;
        let page_route = Route::PageSubmit;
        format!(
            r#"
                <form hx-trigger="change" hx-post="{page_route}">
                    <input type="hidden" name="id" value="{id}" />
                    <input type="hidden" name="collection_id" value="{collection_id}" />
                    <input
                        id="title"
                        name="title"
                        value="{title}"
                        type="text"
                        class="w-[80vw]"
                    />
                </form>
            "#
        )
    }
}

pub struct ContentDisplay<'a> {
    pub content: Option<&'a models::Content>,
    pub page_id: i32,
}
impl Component for ContentDisplay<'_> {
    fn render(&self) -> String {
        let page_id = self.page_id;
        if let Some(content) = self.content {
            let rendered = markdown::to_html(&content.content);
            let cleaned = clean(&rendered);
            let content_route = Route::PageContent(Some(page_id));
            format!(
                r#"
                <div
                    class="prose dark:prose-invert cursor-pointer"
                    hx-get="{content_route}"
                >
                    {cleaned}
                </div>
            "#
            )
        } else {
            let page_content_route = Route::PageContent(Some(page_id));
            format!(
                r#"
                <div
                    class="prose dark:prose-invert cursor-pointer"
                    hx-get="{page_content_route}"
                >
                    <p>Click to add content!</p>
                </div>
            "#
            )
        }
    }
}

impl Component for models::Content {
    fn render(&self) -> String {
        let page_id = self.page_id;
        let content = &self.content;
        let page_content_route = Route::PageContent(Some(page_id));
        // next thing to do is handle submission of this form!
        format!(
            r#"
                <form hx-trigger="change" hx-post="{page_content_route}">
                    <textarea
                        name="content"
                        class="w-[80vw] h-48"
                        >{content}</textarea>
                </form>
            "#
        )
    }
}

impl Component for models::PvBool {
    fn render(&self) -> String {
        let page_id = self.page_id;
        let prop_id = self.prop_id;
        if self.value.is_none() {
            let route = Route::PageNewBoolProp(Some((page_id, prop_id)));
            return NullPropvalButton {
                post_href: &route.as_string(),
            }
            .render();
        };
        let value = self.value.unwrap();
        let checked_state = if value { "checked" } else { "" };
        let route = Route::PageBoolProp(Some((page_id, prop_id)));
        format!(
            r#"
                <input
                    hx-post="{route}"
                    class="justify-self-center"
                    name="value"
                    type="checkbox"
                    {checked_state}
                />
            "#
        )
    }
}

impl Component for models::PvInt {
    fn render(&self) -> String {
        let page_id = self.page_id;
        let prop_id = self.prop_id;
        if self.value.is_none() {
            let route = Route::PageNewIntProp(Some((page_id, prop_id)));
            return NullPropvalButton {
                post_href: &route.as_string(),
            }
            .render();
        };
        let value = self.value.unwrap();
        let route = Route::PageIntProp(Some((page_id, prop_id)));
        format!(
            r#"
                <input
                    class="rounded text-sm w-24 justify-self-center"
                    hx-post="{route}"
                    name="value"
                    type="number"
                    value={value}
                />
            "#
        )
    }
}

impl Component for models::PvFloat {
    fn render(&self) -> String {
        let page_id = self.page_id;
        let prop_id = self.prop_id;
        if self.value.is_none() {
            let route = Route::PageNewFloatProp(Some((page_id, prop_id)));
            return NullPropvalButton {
                post_href: &route.as_string(),
            }
            .render();
        };
        let value = self.value.unwrap();
        let route = Route::PageFloatProp(Some((page_id, prop_id)));
        format!(
            r#"
                <input
                    class="rounded text-sm w-24 justify-self-center"
                    hx-post="{route}"
                    name="value"
                    type="number"
                    step="0.01"
                    value={value}
                />
            "#
        )
    }
}

pub struct NewPage {
    pub collection_id: i32,
    pub page_id: Option<i32>,
    pub title: Option<String>,
}
impl Component for NewPage {
    fn render(&self) -> String {
        let page_id = if let Some(pid) = self.page_id {
            format!(r#"<input type="hidden" name="id" value="{pid}" />"#)
        } else {
            "".to_string()
        };
        let title = if let Some(t) = &self.title {
            clean(t)
        } else {
            "".to_string()
        };
        let collection_route = Route::Collection(Some(self.collection_id));
        let back_button = format!(
            r#"
                <a class="block mb-2 link" href="{collection_route}">Back</a>
            "#
        );
        format!(
            r#"
                {back_button}
                <form hx-post="{collection_route}">
                    <h1 class="text-xl">New Page</h1>
                    <label for="title">Title</label>
                    <input class="rounded" type="text" name="title" id="title" value="{title}" />
                    {page_id}
                    <button class="dark:bg-slate-700 dark:text-white dark:hover:bg-slate-600 transition shadow hover:shadow-none rounded p-1 block">Save</button>
                </form>
            "#,
        )
    }
}

pub struct ArrowUp;
impl Component for ArrowUp {
    fn render(&self) -> String {
        r#"
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 rotate-180">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 13.5L12 21m0 0l-7.5-7.5M12 21V3" />
            </svg>
            "#.to_string()
    }
}

pub struct ArrowDown;
impl Component for ArrowDown {
    fn render(&self) -> String {
        r#"
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 13.5L12 21m0 0l-7.5-7.5M12 21V3" />
            </svg>
            "#.to_string()
    }
}

pub struct PropOrderForm {
    pub props: Vec<models::Prop>,
}
impl Component for PropOrderForm {
    fn render(&self) -> String {
        if self.props.is_empty() {
            return "<p>No props in this workspace!</p>".into();
        };
        let collection_id = self.props[0].collection_id;
        let list_items = self
            .props
            .iter()
            .map(|p| {
                let name = clean(&p.name);
                let pid = p.id;
                let cid = p.collection_id;
                let up = ArrowUp {}.render();
                let down = ArrowDown {}.render();
                let up_route =
                    Route::CollectionIncrementPropOrder(Some((cid, pid)));
                let down_route =
                    Route::CollectionDecrementPropOrder(Some((cid, pid)));
                format!(
                    r##"
                    <li class="flex">
                        <span class="w-48 truncate">{name}</span>
                        <a
                            hx-post="{up_route}"
                            hx-target="closest div"
                            hx-sync="closest ol:queue">{up}</a>
                        <a 
                            hx-post="{down_route}"
                            hx-target="closest div"
                            hx-sync="closest ol:queue">{down}</a>
                    </li>
                    "##
                )
            })
            .collect::<Vec<String>>()
            .join("\n");

        let collection_route = Route::Collection(Some(collection_id));
        format!(
            r#"
                <div>
                    <a class="link" href="{collection_route}">Back</a>
                    <ol class="ml-4 list-decimal"">{list_items}</ol>
                </div>
            "#
        )
    }
}

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
        let collection_id = self.collection_id;
        format!(
            r#"
            <div
                hx-get="/collection/{collection_id}/show-filter-toolbar"
                hx-trigger="toggle-filter-toolbar from:body"
                ></div>
            "#
        )
    }
}

pub struct FilterToolbar<'a> {
    pub bool_filters: Vec<models::FilterBool>,
    pub int_filters: Vec<models::FilterInt>,
    pub int_rng_filters: Vec<models::FilterIntRng>,
    pub float_filters: Vec<models::FilterFloat>,
    pub float_rng_filters: Vec<models::FilterFloatRng>,
    pub collection_id: i32,
    pub get_prop_name: &'a dyn Fn(i32) -> &'a str,
}
impl Component for FilterToolbar<'_> {
    fn render(&self) -> String {
        let collection_id = self.collection_id;
        let bool_rendered =
            self.bool_filters
                .iter()
                .fold(String::new(), |mut acc, filter| {
                    acc.push_str(
                        &FilterBool {
                            filter,
                            prop_name: (self.get_prop_name)(filter.prop_id),
                        }
                        .render_chip(),
                    );
                    acc
                });
        let int_rendered =
            self.int_filters
                .iter()
                .fold(String::new(), |mut acc, filter| {
                    acc.push_str(
                        &FilterInt {
                            filter,
                            prop_name: (self.get_prop_name)(filter.prop_id),
                        }
                        .render_chip(),
                    );
                    acc
                });
        let int_rng_rendered = self.int_rng_filters.iter().fold(
            String::new(),
            |mut acc, filter| {
                acc.push_str(
                    &FilterIntRng {
                        filter,
                        prop_name: (self.get_prop_name)(filter.prop_id),
                    }
                    .render_chip(),
                );
                acc
            },
        );
        let float_rendered =
            self.float_filters
                .iter()
                .fold(String::new(), |mut acc, filter| {
                    acc.push_str(
                        &FilterFloat {
                            filter,
                            prop_name: (self.get_prop_name)(filter.prop_id),
                        }
                        .render_chip(),
                    );
                    acc
                });
        let float_rng_rendered = self.float_rng_filters.iter().fold(
            String::new(),
            |mut acc, filter| {
                acc.push_str(
                    &FilterFloatRng {
                        filter,
                        prop_name: (self.get_prop_name)(filter.prop_id),
                    }
                    .render_chip(),
                );
                acc
            },
        );
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
            {bool_rendered}{int_rendered}{int_rng_rendered}{float_rendered}{float_rng_rendered}
            </div>
            "#
        )
    }
}

pub trait FilterUi {
    fn render_chip(&self) -> String;
    fn render_form(&self) -> String;
}

pub struct FilterBool<'a> {
    pub filter: &'a models::FilterBool,
    pub prop_name: &'a str,
}
impl FilterUi for FilterBool<'_> {
    fn render_chip(&self) -> String {
        let prop_name = self.prop_name;
        let href = Route::FilterBool(Some(self.filter.id));
        let operation_name = self.filter.r#type.get_display_name();
        let val = if let models::FilterType::IsEmpty(..) = self.filter.r#type {
            ""
        } else if self.filter.value {
            r#""true""#
        } else {
            r#""false""#
        };
        let predicate = Box::new(Text { inner_text: &val });
        // Clippy is whining but the borrow-checker will whine louder
        let result = FilterChip {
            subject: prop_name,
            operator_text: operation_name,
            predicate,
            // The same route is used for POST & delete
            form_href: &href.as_string(),
            delete_href: &href.as_string(),
            filter_type: &self.filter.r#type,
        }
        .render();

        result
    }
    fn render_form(&self) -> String {
        let filter_id = self.filter.id;
        let container_id = format!("filter-{filter_id}-form");
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let prop_name = self.prop_name;
        let submit_url = Route::FilterBool(Some(self.filter.id));
        let chip_route = Route::FilterBoolChip(Some(self.filter.id));
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

#[derive(Debug)]
pub struct FilterInt<'a> {
    pub filter: &'a models::FilterInt,
    pub prop_name: &'a str,
}
impl FilterUi for FilterInt<'_> {
    fn render_chip(&self) -> String {
        let href = Route::FilterInt(Some(self.filter.id));
        let prop_name = self.prop_name;
        let operation_name = self.filter.r#type.get_display_name();
        let result = FilterChip {
            subject: prop_name,
            operator_text: operation_name,
            predicate: Box::new(Text {
                inner_text: &self.filter.value,
            }),
            // The same href is used for POST & DELETE
            form_href: &href.as_string(),
            delete_href: &href.as_string(),
            filter_type: &self.filter.r#type,
        }
        .render();

        result
    }
    fn render_form(&self) -> String {
        let value = self.filter.value;
        let prop_name = self.prop_name;
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let type_1_selected =
            if let models::FilterType::Eq(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_2_selected =
            if let models::FilterType::Neq(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_3_selected =
            if let models::FilterType::Gt(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_4_selected =
            if let models::FilterType::Lt(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_7_selected =
            if let models::FilterType::IsEmpty(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let form_route = Route::FilterInt(Some(self.filter.id));
        let chip_route = Route::FilterIntChip(Some(self.filter.id));
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
                        <label class="text-sm" for="type">Filter Type</label>
                        <select
                            id="type"
                            name="type"
                            class="dark:text-white text-sm dark:bg-slate-700 rounded"
                            >
                            <option {type_1_selected} value="1">Exactly Equals</option>
                            <option {type_2_selected} value="2">Does not Equal</option>
                            <option {type_3_selected} value="3">Is Greater Than</option>
                            <option {type_4_selected} value="4">Is Less Than</option>
                            <option {type_7_selected} value="7">Is Empty</option>
                        </select>
                    </div>
                    <div>
                        <label for="value">Value</label>
                        <input id="value" name="value" type="number" value="{value}" />
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

#[derive(Debug)]
pub struct FilterIntRng<'a> {
    pub filter: &'a models::FilterIntRng,
    pub prop_name: &'a str,
}
impl FilterUi for FilterIntRng<'_> {
    fn render_chip(&self) -> String {
        let prop_name = self.prop_name;
        let href = Route::FilterIntRng(Some(self.filter.id));
        let operation_name = self.filter.r#type.get_display_name();
        let start = self.filter.start;
        let end = self.filter.end;
        let predicate = FilterRngPredicate {
            start: Numbah::Int(start),
            end: Numbah::Int(end),
        };
        let result = FilterChip {
            subject: prop_name,
            operator_text: operation_name,
            predicate: Box::new(predicate),
            form_href: &href.as_string(),
            delete_href: &href.as_string(),
            filter_type: &self.filter.r#type,
        }
        .render();

        result
    }
    fn render_form(&self) -> String {
        let prop_name = self.prop_name;
        let start = self.filter.start;
        let end = self.filter.end;
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let type_5_selected = match self.filter.r#type {
            models::FilterType::InRng(_) => "selected",
            _ => "",
        };
        let type_6_selected = match self.filter.r#type {
            models::FilterType::NotInRng(_) => "selected",
            _ => "",
        };
        let form_route = Route::FilterIntRng(Some(self.filter.id));
        let chip_route = Route::FilterIntRngChip(Some(self.filter.id));
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
                    <div>
                        <label class="text-sm" for="type">Filter Type</label>
                        <select
                            id="type"
                            name="type"
                            class="dark:text-white text-sm dark:bg-slate-700 rounded"
                            >
                                <option {type_5_selected} value="5">Is Inside Range</option>
                                <option {type_6_selected} value="6">Is Not Inside Range</option>
                        </select>
                    </div>
                    <div class="flex flex-col">
                        <label for="start">Start</label>
                        <input id="start" name="start" type="number" value="{start}" />
                    </div>
                    <div class="flex flex-col">
                        <label for="end">End</label>
                        <input id="end" name="end" type="number" value="{end}" />
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

#[derive(Debug)]
pub struct FilterFloat<'a> {
    pub filter: &'a models::FilterFloat,
    pub prop_name: &'a str,
}
impl FilterUi for FilterFloat<'_> {
    fn render_chip(&self) -> String {
        let href = Route::FilterFloat(Some(self.filter.id));
        let prop_name = self.prop_name;
        let operation_name = self.filter.r#type.get_display_name();
        let result = FilterChip {
            subject: prop_name,
            operator_text: operation_name,
            predicate: Box::new(Text {
                inner_text: &self.filter.value,
            }),
            // The same href is used for POST & DELETE
            form_href: &href.as_string(),
            delete_href: &href.as_string(),
            filter_type: &self.filter.r#type,
        }
        .render();

        result
    }
    fn render_form(&self) -> String {
        let value = self.filter.value;
        let prop_name = self.prop_name;
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let type_1_selected =
            if let models::FilterType::Eq(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_2_selected =
            if let models::FilterType::Neq(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_3_selected =
            if let models::FilterType::Gt(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_4_selected =
            if let models::FilterType::Lt(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let type_7_selected =
            if let models::FilterType::IsEmpty(..) = self.filter.r#type {
                "selected"
            } else {
                ""
            };
        let form_route = Route::FilterFloat(Some(self.filter.id));
        let chip_route = Route::FilterFloatChip(Some(self.filter.id));
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
                        <label class="text-sm" for="type">Filter Type</label>
                        <select
                            id="type"
                            name="type"
                            class="dark:text-white text-sm dark:bg-slate-700 rounded"
                            >
                            <option {type_1_selected} value="1">Exactly Equals</option>
                            <option {type_2_selected} value="2">Does not Equal</option>
                            <option {type_3_selected} value="3">Is Greater Than</option>
                            <option {type_4_selected} value="4">Is Less Than</option>
                            <option {type_7_selected} value="7">Is Empty</option>
                        </select>
                    </div>
                    <div>
                        <label for="value">Value</label>
                        <input id="value" name="value" type="number" step="0.01" value="{value}" />
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

#[derive(Debug)]
pub struct FilterFloatRng<'a> {
    pub filter: &'a models::FilterFloatRng,
    pub prop_name: &'a str,
}
impl FilterUi for FilterFloatRng<'_> {
    fn render_chip(&self) -> String {
        let prop_name = self.prop_name;
        let href = Route::FilterFloatRng(Some(self.filter.id));
        let operation_name = self.filter.r#type.get_display_name();
        let start = self.filter.start;
        let end = self.filter.end;
        let predicate = FilterRngPredicate {
            start: Numbah::Float(start),
            end: Numbah::Float(end),
        };
        let result = FilterChip {
            subject: prop_name,
            operator_text: operation_name,
            predicate: Box::new(predicate),
            form_href: &href.as_string(),
            delete_href: &href.as_string(),
            filter_type: &self.filter.r#type,
        }
        .render();

        result
    }
    fn render_form(&self) -> String {
        let prop_name = self.prop_name;
        let start = self.filter.start;
        let end = self.filter.end;
        let chevron = Chevron {
            variant: ChevronVariant::Open,
        }
        .render();
        let type_5_selected = match self.filter.r#type {
            models::FilterType::InRng(_) => "selected",
            _ => "",
        };
        let type_6_selected = match self.filter.r#type {
            models::FilterType::NotInRng(_) => "selected",
            _ => "",
        };
        let form_route = Route::FilterFloatRng(Some(self.filter.id));
        let chip_route = Route::FilterFloatRngChip(Some(self.filter.id));
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
                    <div>
                        <label class="text-sm" for="type">Filter Type</label>
                        <select
                            id="type"
                            name="type"
                            class="dark:text-white text-sm dark:bg-slate-700 rounded"
                            >
                                <option {type_5_selected} value="5">Is Inside Range</option>
                                <option {type_6_selected} value="6">Is Not Inside Range</option>
                        </select>
                    </div>
                    <div class="flex flex-col">
                        <label for="start">Start</label>
                        <input id="start" name="start" type="number" step="0.01" value="{start}" />
                    </div>
                    <div class="flex flex-col">
                        <label for="end">End</label>
                        <input id="end" name="end" type="number" step="0.01" value="{end}" />
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

enum ChevronVariant {
    #[allow(dead_code)]
    Open,
    Closed,
}
struct Chevron {
    variant: ChevronVariant,
}
impl Component for Chevron {
    fn render(&self) -> String {
        match self.variant {
            ChevronVariant::Open => r#"
                <div class="w-6 h-6">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
                </svg>
                </div>
            "#.into()
            ,
            ChevronVariant::Closed => r#"
                <div class="w-6 h-6">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" />
                </svg>
                </div>
            "#.into()
        }
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

struct Text<'a, T: Display> {
    inner_text: &'a T,
}
impl<T: Display> Component for Text<'_, T> {
    fn render(&self) -> String {
        clean(&format!("{}", self.inner_text))
    }
}

enum Numbah {
    Float(f64),
    Int(i64),
}
impl std::fmt::Display for Numbah {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Float(numbahhh) => write!(f, "{}", numbahhh),
            Self::Int(nuuuuummmmmmmbahhhhh) => {
                write!(f, "{}", nuuuuummmmmmmbahhhhh)
            }
        }
    }
}

struct FilterRngPredicate {
    start: Numbah,
    end: Numbah,
}
impl Component for FilterRngPredicate {
    fn render(&self) -> String {
        let start = &self.start;
        let end = &self.end;
        let arrow_icon = ArrowLeftRight {}.render();

        format!("{start} {arrow_icon} {end}")
    }
}

const FILTER_CONTAINER_STYLE: &str = "max-w-sm text-sm border border-slate-600 bg-gradient-to-tr from-blue-100 to-fuchsia-100 dark:bg-gradient-to-tr dark:from-fuchsia-800 dark:to-violet-700 rounded p-2 flex flex-row gap-2 items-center justify-center";

struct FilterChip<'a> {
    pub subject: &'a str,
    pub operator_text: &'a str,
    pub predicate: Box<dyn Component + 'a>,
    pub form_href: &'a str,
    pub delete_href: &'a str,
    pub filter_type: &'a models::FilterType,
}
impl Component for FilterChip<'_> {
    fn render(&self) -> String {
        let form_href = self.form_href;
        let subject = clean(self.subject);
        let operator_text = clean(self.operator_text);
        let child = if let models::FilterType::IsEmpty(..) = self.filter_type {
            "".into()
        } else {
            self.predicate.render()
        };
        let chevron = Chevron {
            variant: ChevronVariant::Closed,
        }
        .render();
        let delete_btn = DeleteButton {
            delete_href: self.delete_href,
            hx_target: Some("closest div"),
        }
        .render();
        format!(
            r#"
            <div
                hx-get="{form_href}"
                class="{FILTER_CONTAINER_STYLE} h-10 cursor-pointer">
                {chevron}
                <span class="text-xs">{subject}</span>
                <span class="text-[10px] bg-slate-100 bg-opacity-40 rounded text-black p-1 shadow italic whitespace-nowrap">{operator_text}</span>
                {child}
                {delete_btn}
            </div>
            "#
        )
    }
}

pub struct ChoosePropForFilter<'a> {
    pub props: &'a Vec<&'a models::Prop>,
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
                    models::PropValTypes::Int => "integer",
                    models::PropValTypes::Bool => "checkbox",
                    models::PropValTypes::Float => "percent",
                    _ => todo!(),
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

fn get_slug(
    filter_type: &models::FilterType,
    prop_type: &models::PropValTypes,
) -> String {
    let filter_type_id = filter_type.get_int_repr();
    match filter_type {
        models::FilterType::Neq(_)
        | models::FilterType::Lt(_)
        | models::FilterType::Gt(_)
        | models::FilterType::Eq(_) => match prop_type {
            models::PropValTypes::Bool => {
                format!("new-bool-filter?type_id={filter_type_id}")
            }
            models::PropValTypes::Int => {
                format!("new-int-filter?type_id={filter_type_id}")
            }
            models::PropValTypes::Float => {
                format!("new-float-filter?type_id={filter_type_id}")
            }
            _ => todo!(),
        },
        models::FilterType::InRng(_) | models::FilterType::NotInRng(_) => {
            match prop_type {
                models::PropValTypes::Bool => {
                    panic!("in-rng and not-in-rng not supported for bool")
                }
                models::PropValTypes::Int => {
                    format!("new-int-rng-filter?type_id={filter_type_id}")
                }
                models::PropValTypes::Float => {
                    format!("new-float-rng-filter?type_id={filter_type_id}")
                }
                _ => todo!(),
            }
        }
        models::FilterType::IsEmpty(_) => match prop_type {
            models::PropValTypes::Bool => {
                format!("new-bool-filter?type_id={filter_type_id}")
            }
            models::PropValTypes::Int => {
                format!("new-int-filter?type_id={filter_type_id}")
            }
            models::PropValTypes::Float => {
                format!("new-float-filter?type_id={filter_type_id}")
            }
            _ => todo!(),
        },
    }
}

pub struct NewFilterTypeOptions<'a> {
    pub options: &'a Vec<models::FilterType>,
    pub prop_id: i32,
    pub prop_type: &'a models::PropValTypes,
}
impl Component for NewFilterTypeOptions<'_> {
    fn render(&self) -> String {
        let button_style = "p-2 w-full text-md rounded dark:bg-blue-700 dark:hover:bg-blue-600 shadow hover:shadow-none";
        let prop_id = self.prop_id;
        let rendered_options =
            self.options.iter().fold(String::new(), |mut str, opt| {
                let opt_text = clean(opt.get_display_name());
                let type_slug = get_slug(opt, self.prop_type);
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

struct DeleteButton<'a> {
    delete_href: &'a str,
    hx_target: Option<&'a str>,
}
impl Component for DeleteButton<'_> {
    fn render(&self) -> String {
        let delete_href = self.delete_href;
        let hx_target = if let Some(t) = self.hx_target {
            format!(r#"hx-target="{t}""#)
        } else {
            "".into()
        };

        // Note: we're stopping propagation on the button since it's inside
        // the chip div which itself has a hx-get. If the event propagates up
        // to the div, both are triggered, and then the swap happens according
        // to whichever one wins the race.
        format!(
            r#"
            <button
                onclick="(arguments[0] || window.event).stopPropagation();"
                hx-delete="{delete_href}"
                {hx_target}
                >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 bg-red-500 rounded-full">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M9.75 9.75l4.5 4.5m0-4.5l-4.5 4.5M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
            </button>
            "#
        )
    }
}

pub struct Div<'a> {
    pub class: &'a str,
    pub hx_get: Option<&'a str>,
    pub hx_trigger: Option<&'a str>,
    pub children: Box<dyn Component>,
}
impl Component for Div<'_> {
    fn render(&self) -> String {
        let hx_get = if let Some(hxg) = self.hx_get {
            clean(&format!(r#"hx-get="{hxg}""#))
        } else {
            "".into()
        };
        let hx_trigger = if let Some(hxt) = self.hx_trigger {
            clean(&format!(r#"hx-trigger="{hxt}""#))
        } else {
            "".into()
        };
        let class = self.class;
        let children = self.children.render();
        format!(
            r#"<div {hx_get} {hx_trigger} class="{class}">{children}</div>"#
        )
    }
}

pub struct SortIcon {
    pub collection_id: i32,
}
impl Component for SortIcon {
    fn render(&self) -> String {
        r##"
        <div id="sort-icon" class="rounded">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6">
              <path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5M12 17.25h8.25" />
            </svg>
        </div>
        <script>
            (() => {{
            const iconElement = document.querySelector("#sort-icon");
            iconElement.addEventListener('click', function () {{
                if ([...iconElement.classList].includes('text-black')) {{
                    iconElement.classList.remove('text-black');
                    iconElement.classList.remove('bg-yellow-100');
                    htmx.trigger('body', 'toggle-sort-toolbar');
                }} else {{
                    iconElement.classList.add('text-black');
                    iconElement.classList.add('bg-yellow-100');
                    htmx.trigger('body', 'toggle-sort-toolbar');
                }}
            }});
            }})()
        </script>
        "##.into()
    }
}

pub struct SortToolbar<'a> {
    pub collection_id: i32,
    pub default_selected_prop: Option<i32>,
    pub prop_choices: &'a [models::Prop],
    pub sort_type: Option<models::SortType>,
}
impl Component for SortToolbar<'_> {
    fn render(&self) -> String {
        let collection_id = self.collection_id;
        let disable_sorting_option = if self.default_selected_prop.is_some() {
            r#"<option selected value="-1">-- Disable Sorting --</option>"#
        } else {
            r#"<option value="-1">-- Disable Sorting --</option>"#
        };
        let prop_options =
            self.prop_choices
                .iter()
                .fold(String::new(), |mut str, prop| {
                    let is_selected = if let Some(selected_prop) =
                        self.default_selected_prop
                    {
                        if prop.id == selected_prop {
                            "selected"
                        } else {
                            ""
                        }
                    } else {
                        ""
                    };
                    let prop_id = prop.id;
                    let prop_name = clean(&prop.name);
                    str.push_str(&format!(
                r#"<option {is_selected} value={prop_id}>{prop_name}</option>"#
            ));
                    str
                });
        let sort_order_options = match self.sort_type {
            Some(ref t) => match t {
                models::SortType::Asc => {
                    r#"<option selected value="1">Ascending</option>
                   <option value="2">Descending</option>"#
                }
                models::SortType::Desc => {
                    r#"<option value="1">Ascending</option>
                   <option selected value="2">Descending</option>"#
                }
            },
            None => {
                r#"<option value="1">Ascending</option>
                   <option value="2">Descending</option>"#
            }
        };
        let hide_toolbar =
            Route::CollectionHideSortToolbar(Some(collection_id));
        let sort_route = Route::CollectionSort(Some(collection_id));
        format!(
            r#"<div
                hx-get="{hide_toolbar}"
                hx-trigger="toggle-sort-toolbar from:body"
                >
                    <form
                        class="my-2"
                        hx-post="{sort_route}"
                        >
                        <select
                            name="sort_by"
                            class="dark:text-white text-sm dark:bg-slate-700 rounded"
                            >
                            {disable_sorting_option}
                            {prop_options}
                        </select>
                        <select
                            name="sort_order"
                            class="dark:text-white text-sm dark:bg-slate-700 rounded"
                        >
                            {sort_order_options}
                        </select>
                        <button class="dark:bg-slate-700 dark:text-white dark:hover:bg-slate-600 transition shadow hover:shadow-none rounded p-1">Save</button>
                    </form>
                </div>
            "#
        )
    }
}

pub struct SortToolbarPlaceholder {
    pub collection_id: i32,
}
impl Component for SortToolbarPlaceholder {
    fn render(&self) -> String {
        let route = Route::CollectionShowSortToolbar(Some(self.collection_id));
        format!(
            r#"
            <div
                hx-get="{route}"
                hx-trigger="toggle-sort-toolbar from:body"
                ></div>
            "#
        )
    }
}

pub struct SortOrderSavedConfirmation {
    pub collection_id: i32,
}
impl Component for SortOrderSavedConfirmation {
    fn render(&self) -> String {
        let route = Route::CollectionHideSortToolbar(Some(self.collection_id));
        format!(
            r##"
            <div
                hx-get="{route}"
                hx-trigger="load delay:3s"
                class="my-2"
                >
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="inline bg-green-800 p-2 rounded-full w-8 h-8">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
                </svg>
                Sort order was saved
                <script>
                    setTimeout(() => {{
                        const iconElement = document.querySelector("#sort-icon");
                        iconElement.classList.remove('text-black');
                        iconElement.classList.remove('bg-yellow-100');
                        htmx.trigger('body', 'toggle-sort-toolbar');
                    }}, 2000);
                </script>
            </div>
            "##
        )
    }
}

pub struct NullPropvalButton<'a> {
    pub post_href: &'a str,
}
impl Component for NullPropvalButton<'_> {
    fn render(&self) -> String {
        let post_href = self.post_href;
        format!(
            r#"
            <button
                hx-get="{post_href}"
                >--</button>
            "#
        )
    }
}

pub struct LoginForm;
impl Component for LoginForm {
    fn render(&self) -> String {
        let login_route = Route::Login;
        format!(
            r#"
            <form class="flex flex-col gap-2 max-w-md" hx-post="{login_route}">
                <h1 class="text-xl">Login</h1>
                <label autocomplete="username" for="identifier">Username or Email</label>
                <input type="text" id="identifier" name="identifier" />
                <label for="passwored">Password</label>
                <input autocomplete="current-password" type="password" id="password" name="password" />
                <button class="dark:bg-slate-700 w-36 dark:text-white dark:hover:bg-slate-600 transition shadow hover:shadow-none rounded p-1 block">Log In</button>
            </form>
            "#
        )
    }
}

pub struct RegisterForm;
impl Component for RegisterForm {
    fn render(&self) -> String {
        let register_route = Route::Register;
        format!(
            r#"
            <form class="flex flex-col gap-2 max-w-md" hx-post="{register_route}">
                <h1 class="text-xl">Register for an Account</h1>
                <label for="username">Username</label>
                <input autocomplete="username" type="text" id="username" name="username" />
                <label for="email">Email</label>
                <input type="email" id="email" name="email" />
                <label for="password">Password</label>
                <input autocomplete="current-password" type="password" id="password" name="password" />
                <label for="secret_word">Secret Word</label>
                <p class="text-sm dark:text-slate-100">
                    What is the secret word? This app is under development and
                    this is how I will prevent login spam, though you may take
                    a look at the source code and find the secret word if you
                    really want a sneak peek so bad. Let reading the source
                    be your Captcha.
                </p>
                <input type="text" id="secret_word" name="secret_word" />
                <button class="dark:bg-slate-700 w-36 dark:text-white dark:hover:bg-slate-600 transition shadow hover:shadow-none rounded p-1 block">Sign Up</button>
            </form>
            "#
        )
    }
}

pub struct ExternalLink<'a> {
    pub href: &'a str,
    pub children: Box<dyn Component>,
}
impl Component for ExternalLink<'_> {
    fn render(&self) -> String {
        let children = self.children.render();
        let href = self.href;
        format!(
            r#"
            <a href={href}>
                {children}
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3 h-3 inline">
                  <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 19.5l15-15m0 0H8.25m11.25 0v11.25" />
                </svg>
            </a>
            "#
        )
    }
}
