use super::models;
use ammonia::clean;
use std::fmt::Write;

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
    pub title: String,
    pub children: Box<dyn Component + 'a>,
}

impl Component for Page<'_> {
    fn render(&self) -> String {
        // note: we'll get a compiler error here until the tailwind build occurs.
        // Make sure you use `make build` in the Makefile to get both to happen
        // together
        let tailwind = include_str!("./tailwind.generated.css");
        format!(
            r#"
            <html>
                <head>
                    <meta name="viewport" content="width=device-width, initial-scale=1.0"></meta>
                    <title>{title}</title>
                    <style>
                        {tailwind}
                    </style>
                </head>
                <body hx-boost="true" class="dark:bg-indigo-1000 dark:text-white mt-2 ml-2 sm:mt-8 sm:ml-8">
                    {body_html}
                    <script src="/static/htmx-1.9.4"></script>
                    {LIVE_RELOAD_SCRIPT}
                </body>
            </html>
            "#,
            tailwind = tailwind,
            title = clean(&self.title),
            body_html = self.children.render()
        )
    }
}

#[derive(Default)]
pub struct TodoHome {}
impl Component for TodoHome {
    fn render(&self) -> String {
        r##"
        <main class="flex items-center justify-center">
            <div class="max-w-md p-2 m-2 bg-indigo-50 rounded shadow">
                <h1 class="text-xl mb-4 text-center">Todo App</h1>
                <form
                    class="flex flex-row"
                    hx-post="/item"
                    hx-target="#todo-items"
                    hx-swap="afterbegin"
                    hx-on::after-request="this.reset()"
                >
                    <div class="flex items-center gap-2">
                        <label for="title">Add Item</label>
                        <input class="rounded" type="text" name="title" id="title" />
                        <button class="w-24 h-8 bg-blue-200 rounded shadow hover:shadow-none hover:bg-blue-300 hover:font-bold transition">Submit</button>
                    </div>
                </form>
                <div
                    hx-get="/item"
                    hx-trigger="load"
                    id="todo-items"
                >
                    Loading your todo list...
                </div>
            </div>
        </main>
        "##.to_string()
    }
}

pub struct Item {
    pub item: models::Item,
}
impl Component for Item {
    fn render(&self) -> String {
        let checked_state = if self.item.is_completed {
            "checked"
        } else {
            ""
        };
        let id_str = if let Some(id) = self.item.id {
            format!("{}", id)
        } else {
            "".to_string()
        };
        format!(
            r#"
            <form
                class="rounded flex items-center gap-2"
            >
                <input
                    class="rounded"
                    type="checkbox" {checked_state} 
                    name="is_completed"
                    hx-post="/item"
                    hx-target="closest form"
                    hx-swap="outerHTML"
                />
                <input type="hidden" name="title" value="{title}" />
                <input type="hidden" name="id" value="{id}" />
                <h2 class="text-md">{title}</h2>
                <button
                    hx-delete="/item/{id}"
                    hx-swap="outerHTML"
                    hx-target="closest form"
                    class="flex items-center justify-center rounded-full text-lg
                    w-6 h-6 bg-red-100 justify-self-right"
                >x</button>
            </form>
            "#,
            id = id_str,
            title = clean(&self.item.title),
            checked_state = checked_state
        )
    }
}

pub struct ItemList {
    pub items: Vec<models::Item>,
    pub next_page: Option<i32>,
}
impl Component for ItemList {
    fn render(&self) -> String {
        let mut items_clone = self.items.clone();
        items_clone.sort_by_key(|i| i.is_completed);
        let items = items_clone
            .iter()
            .map(|i| Item { item: i.clone() }.render())
            .collect::<Vec<String>>()
            .join("");
        let hx_get_infinite_scroll = if let Some(page) = self.next_page {
            InfiniteScroll {
                next_href: format!("/item?page={}", page),
            }
            .render()
        } else {
            "".to_string()
        };

        if !items_clone.is_empty() {
            [items, hx_get_infinite_scroll].join("")
        } else {
            "".to_string()
        }
    }
}

pub struct InfiniteScroll {
    pub next_href: String,
}
impl Component for InfiniteScroll {
    fn render(&self) -> String {
        format!(
            r#"<div hx-trigger="revealed" hx-swap="outerHTML" hx-get="{}" />"#,
            clean(&self.next_href)
        )
    }
}

pub struct Collection {
    pub id: i32,
    pub name: String,
}
impl Component for Collection {
    fn render(&self) -> String {
        format!(
            r#"
            <h1 class="serif text-xl my-4">{name}</h1>
            <a class="link" href="/collection/{id}/new-page">Create Page</a>
            <main hx-trigger="load" hx-get="/collection/{id}/list-pages">Loading Pages...</main>
        "#,
            id = self.id,
            name = clean(&self.name)
        )
    }
}

pub struct PageList<'a> {
    pub pages: &'a [models::Page],
}
impl Component for PageList<'_> {
    fn render(&self) -> String {
        let list = self.pages.iter().fold(String::new(), |mut str, page| {
            let _ = write!(
                str,
                r#"
                        <div class="flex gap-2 my-1 items-center">
                            <a class="link" href="/page/{page_id}">Edit</a>
                            <div><div class="w-64 truncate">{title}</div></div>
                            {other_props}
                        </div>
                    "#,
                page_id = page.id,
                title = clean(&page.title),
                other_props = page
                    .props
                    .iter()
                    .map(|p| p.render())
                    .collect::<Vec<String>>()
                    .join("")
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
        let col_order = HoverIcon {
            children: Box::new(ColumnOrderIcon {
                collection_id: self.pages[0].collection_id,
            }),
            tooltip_text: "Edit Column Order",
        }
        .render();
        format!(
            r#"
                <div class="mt-2">
                    {col_order}
                </div>
                <div class="overflow-y-scroll">
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
        format!(
            r#"
                <a href="/collection/{cid}/prop-order">
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
                <div data-tooltip="{text}" class="tooltip">
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
        let back_button = format!(
            r#"
                <a class="block mb-2 link" href="/collection/{}">Back</a>
            "#,
            self.page.collection_id
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
        format!(
            r#"
                <form hx-trigger="change" hx-post="/page" hx-swap="outerHTML">
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
            format!(
                r#"
                <div
                    class="prose dark:prose-invert cursor-pointer"
                    hx-get="/page/{page_id}/content"
                    hx-swap="outerHTML"
                >
                    {cleaned}
                </div>
            "#
            )
        } else {
            format!(
                r#"
                <div
                    class="prose dark:prose-invert cursor-pointer"
                    hx-get="/page/{page_id}/content"
                    hx-swap="outerHTML"
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
        // next thing to do is handle submission of this form!
        format!(
            r#"
                <form hx-trigger="change" hx-post="/page/{page_id}/content">
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
        let checked_state = if self.value { "checked" } else { "" };
        let page_id = self.page_id;
        let prop_id = self.prop_id;
        format!(
            r#"
                <input
                    hx-post="/page/{page_id}/prop/{prop_id}/bool"
                    hx-swap="outerHTML"
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
        let value = self.value;
        format!(
            r#"
                <input
                    class="rounded text-sm w-24"
                    hx-post="/page/{page_id}/prop/{prop_id}/int"
                    hx-swap="outerHTML"
                    name="value"
                    type="number"
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
        let cid = self.collection_id;
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
        let back_button = format!(
            r#"
                <a class="block mb-2 link" href="/collection/{}">Back</a>
            "#,
            self.collection_id
        );
        format!(
            r#"
                {back_button}
                <form hx-post="/collection/{cid}">
                    <h1 class="text-xl">New Page</h1>
                    <label for="title">Title</label>
                    <input class="rounded" type="text" name="title" id="title" value="{title}" />
                    {page_id}
                    <button>Save</button>
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
        let list_items = self
            .props
            .iter()
            .map(|p| {
                let name = clean(&p.name);
                let pid = p.id;
                let cid = p.collection_id;
                let up = ArrowUp {}.render();
                let down = ArrowDown {}.render();
                format!(
                    r##"
                <li class="flex">
                    <span class="w-48 truncate">{name}</span>
                    <a
                        hx-post="/collection/{cid}/prop/{pid}/up"
                        hx-swap="outerHTML"
                        hx-target="closest ol"
                        hx-sync="closest ol:queue">{up}</a>
                    <a 
                        hx-post="/collection/{cid}/prop/{pid}/down"
                        hx-swap="outerHTML"
                        hx-target="closest ol"
                        hx-sync="closest ol:queue">{down}</a>
                </li>
                "##
                )
            })
            .collect::<Vec<String>>()
            .join("\n");
        format!(r#"<ol class="ml-4 list-decimal"">{list_items}</ol>"#)
    }
}
