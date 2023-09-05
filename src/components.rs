use super::models;
use ammonia::clean;

pub trait Component {
    /// Render the component to a HTML string. By convention, the
    /// implementation should sanitize all string properties at render-time
    fn render(&self) -> String;
}

pub struct Page {
    pub title: String,
    pub children: Box<dyn Component>,
}

impl Component for Page {
    fn render(&self) -> String {
        // note: we'll get a compiler error here until the tailwind build occurs.
        // Make sure you use `make build` in the Makefile to get both to happen
        // together
        let tailwind = include_str!("./tailwind.generated.css");
        let htmx = include_str!("./htmx-1.9.4.vendor.js");
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
                <body>
                    {body_html}
                    <script>{htmx}</script>
                </body>
            </html>
            "#,
            tailwind = tailwind,
            htmx = htmx,
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
            <main hx-trigger="load" hx-get="/collection/{id}/pages">Loading Pages...</main>
        "#,
            id = self.id,
            name = clean(&self.name)
        )
    }
}

impl Component for models::Page {
    fn render(&self) -> String {
        format!(
            r#"
                <div class="flex gap-2 my-1 items-center">
                    <div class="w-64 truncate">{title}</div>
                    {other_props}
                </div>
            "#,
            title = clean(&self.title),
            other_props = self
                .props
                .iter()
                .map(|p| p.render())
                .collect::<Vec<String>>()
                .join("")
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
