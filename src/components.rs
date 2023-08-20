use super::models;
use ammonia::clean;

mod private {
    pub trait ComponentInternal {
        fn sanitize(&self) -> Self;
        fn render_internal(sanitized: &Self) -> String;
    }
}

pub trait Component: private::ComponentInternal + Sized + Clone {
    fn render(&self) -> String {
        let sanitized_self = self.sanitize();
        Self::render_internal(&sanitized_self)
    }
}

#[derive(Clone)]
pub struct Page<T>
where
    T: Component,
{
    pub title: String,
    pub children: Box<T>,
}

impl<T> Component for Page<T> where T: Component {}
impl<T> private::ComponentInternal for Page<T>
where
    T: Component,
{
    fn sanitize(&self) -> Page<T> {
        Page {
            title: clean(&self.title),
            children: self.children.clone(),
        }
    }
    fn render_internal(sanitized: &Page<T>) -> String {
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
            title = sanitized.title,
            body_html = sanitized.children.render()
        )
    }
}

#[derive(Default, Clone)]
pub struct TodoHome {}
impl Component for TodoHome {}
impl private::ComponentInternal for TodoHome {
    fn sanitize(&self) -> Self {
        TodoHome::default()
    }
    fn render_internal(_: &TodoHome) -> String {
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

#[derive(Clone)]
pub struct Item {
    pub item: models::Item,
}
impl Component for Item {}
impl private::ComponentInternal for Item {
    fn sanitize(&self) -> Self {
        Item {
            item: models::Item {
                title: clean(&self.item.title),
                id: self.item.id,
                is_completed: self.item.is_completed,
            },
        }
    }
    fn render_internal(sanitized: &Item) -> String {
        let checked_state = if sanitized.item.is_completed {
            "checked"
        } else {
            ""
        };
        let id_str = if let Some(id) = sanitized.item.id {
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
            title = sanitized.item.title,
            checked_state = checked_state
        )
    }
}

#[derive(Clone)]
pub struct ItemList {
    pub items: Vec<models::Item>,
    pub next_page: Option<i32>,
}
impl Component for ItemList {}
impl private::ComponentInternal for ItemList {
    fn sanitize(&self) -> Self {
        // Item component will sanitize
        self.clone()
    }
    fn render_internal(sanitized: &ItemList) -> String {
        let mut items_clone = sanitized.items.clone();
        items_clone.sort_by_key(|i| i.is_completed);
        let items = items_clone
            .iter()
            .map(|i| Item { item: i.clone() }.render())
            .collect::<Vec<String>>()
            .join("");
        let hx_get_infinite_scroll = if let Some(page) = sanitized.next_page {
            InfiniteScroll {
                next_href: format!("/item?page={}", page),
            }
            .render()
        } else {
            "".to_string()
        };

        if items_clone.len() != 0 {
        [items, hx_get_infinite_scroll].join("")
        } else {
            "".to_string()
        }
    }
}

#[derive(Clone)]
pub struct InfiniteScroll {
    pub next_href: String,
}
impl Component for InfiniteScroll {}
impl private::ComponentInternal for InfiniteScroll {
    fn sanitize(&self) -> Self {
        InfiniteScroll {
            next_href: clean(&self.next_href),
        }
    }
    fn render_internal(sanitized: &Self) -> String {
        format!(
            r#"<div hx-trigger="revealed" hx-swap="outerHTML" hx-get="{}" />"#,
            sanitized.next_href
        )
    }
}
