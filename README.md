# Notion Clone!

https://github.com/jdevries3133/nc/assets/58614260/ec0d6d17-0e32-429c-96a7-5ef0ca257c26

# Next Steps

1. Great propval refactor (see [exact next steps](#exact-next-steps))
2. Implement datetime
3. Implement multistr (tags)
4. Paginate the collection list view

# Great Propval Refactor

Alllllllllllllllllllllllllllllll righty. The amount of copying feels untenable.
Introducing a single new propval type is > 1000 LOC and hurts my fingers so so
bad. It's time for a mighty refactor.

## High Level

At a high level, I've realized I should have modeled things like this:

```rust
enum Value {
  Bool(bool),
  Int(i64),
  Float(f64),
  // ... etc
}

struct PropVal {
  page_id: i32,
  prop_id: i32,
  vaue: Value
}

enum FilterType {
  Lt(Value), Gt(Value), Eq(Value), InsideRange((Value, Value)), // ... etc
}

struct Filter {
  collection_id: i32,
  prop_id: i32,
  filter_type: FilterType
}
```

With a model like this, I can do things dynamically for the most part ("on the
outside"), but still do static dispatch while matching on the Value enum ("on
the inside"). In particular, a positive of doing things this way is that I can
still have static "hard-coded" SQLx queries:

```rust
match val {
  Value::Int(i) => {
    sqlx::query_as!(
      "insert into propval_int (page_id, prop_id, value)
      values ($1, $2, $3)",
      page_id,
      prop_id,
      i
    ),
  // ... etc
}
```

## Routing and Form Structures

I'm going to keep all routes the same. In particular, I think that dynamic Axum
form parsing is a PITA, so I want a distinct route and form struct for each
data-type. I did consult our lord and savior ChatGPT to figure out how that
would work, and this is what we came up with:

```rust
async fn handle_form_data(form: Form<HashMap<String, String>>) -> String {
    let form_data = form.into_inner();
    let mut response = String::new();
    for (key, value) in form_data {
        // Attempt to parse the value into different data types
        if let Ok(parsed_int) = value.parse::<i64>() {
            println!("{}: {} (parsed as i64)\n", key, parsed_int));
        } else if let Ok(parsed_float) = value.parse::<f64>() {
            println!("{}: {} (parsed as f64)\n", key, parsed_float));
        } else if let Ok(parsed_bool) = value.parse::<bool>() {
            println!("{}: {} (parsed as bool)\n", key, parsed_bool));
        } else {
            println!("{}: {} (parsed as string)\n", key, value));
        }
    }
}
```

Since I do ultimately only have a single value to dynamically parse in this way,
I think that this strategy in combination with some `type` field (so I know what
I'm expecting to receive) could work. However, this is ultimately the same
amount of semantic complexity as fanning it out over several routes, and I
already have the routes laid out, so it's easier to just maintain what I have
there.

## Everything Else

However, beyond the routing and route-handler layer, the whole PropVal and
filter layer will be a lot more unified. To understand, here is what we have
now:

```
bool  controller --> bool  model --> bool  db_op --> bool  component
int   controller --> int   model --> int   db_op --> int   component
float controller --> float model --> float db_op --> float component
(etc)
```

And currently, filters have a similar architecture

```
bool filter controller --> bool filter model --> bool filter db_op --> bool filter component
int  filter controller --> int  filter model --> int  filter db_op --> int  filter component
```

Instead, after the refactor, everything will be unified under the `Prop` struct,
where `prop.value` will be an instance of the `Value` enum. So, the architecture
for prop-val CRUD and filter CRUD will change to look like this:

```
bool  controller ------------------------|
int   controller ------------->    common model --> common db_op --> common component
float controller ------------------------|
```

Now, of course, the pipeline of common stuff will include plenty of match
expressions over the value enum anytime we need to deal with values, but as long
as the value access and operation patterns are structurally the same, there
should be a lot less code. And, adding additional data-types should be a simple
job. Maybe most importantly, it'll be much more DRY than what we have now. Right
now, there is so much copy-and-paste that any sort of change would be totally
impossible.

## Overview of Changes Required

Ultimately, everything dealing with property values or filters other than routes
and controllers will need to be rewritten. In the end, we will have a single
PropVal data-type (with an inner value enum), and single Filter data-type. Each
of these will implement `DbModel` and `Component` (or `FilterUI` for filters).

Another thing to keep in mind is whether `prop_val.value` should be `Value` or
`Option<Value>`. All propval types have a value of type `Option<T>` whereas
filters typically have `T`. I do prefer how things turned out for filters, since
the end result is decoupling the filter creation flow from the filter data-model
itself. Of course, for filters this ended up being especially smart since the
filter creation flow is a whole tricky user flow. However, I think that for most
object types, keeping creation on the sidelines makes sense.

Then, the natural next question is: what should filtering for PropVals look
like? Well, I am thinking something like this:

```rust
enum NewPv {
    Int,
    Float,
    Date,
    DateTime,
    // ... etc
}

impl Component for NewPv {
    fn render(&self) -> String {
        // render a form for whatever type...
    }
}
```

Then, the submit handler can use the same `Prop` data-model, since we'll have a
value in memory at that point. I do think that I'll keep adding N routes for N
data-types. Though it is annoying to write a lot of routes and controller
functions, it's handy to let Axum do the form parsing statically and
declaratively.

**A summary of changes needed:**

- everything propval or filter-related in `src/models.rs` is getting replaced
- `trait PropVal` is no longer needed, since we'll have `struct PropVal`
- `struct PropVal` will need a (quite lengthy) impl for `DbModel`
- UI components will become generic as well
- controllers will change to dispatch to these new internals
- `db_ops::list_pages` will need some changes, but should be much simpler
  and easier to read afterwards, I hope

## Exact Next Steps

1. define the `PropVal` data-type, and migrate all PvBool functionality to it.
2. migrate PvInf functionality to `PropVal`

# Other Future Ideas

These are in priority order.

- hover tooltips are gross; we can do a better implementation with a bit of JS
- improve UX around hover-tooltip-icons on mobile by changing them to a button
  with the icon and text
- I would like middleware to minify and compress outgoing HTML
- add support for transactions to DbModel
  - we can probably achieve this with:
    ```
    enum Db {
      Db(&PgPool),
      Tx(Transaction<Postgres>)
    }
    ```
- Deal with treating db_op error as "Not Found"
  - we're treating DB errors as not found in some cases
  - formally, this ain't correct; maybe DB ops should return a 3-member enum of
    `Found<T>`, `NotFound`, or `Error<E>`
- it would be nice to order filters by creation date... currently, the order in
  the toolbar is basically nondeterministic; though in practice they'll appear
  first sorted by type and secondarily sorted by order of creation, which is
  fine, I suppose
- we need to query for filters and sorts before the main query for initial page
  load. An in-memory cache for all collection filters and sorts would be awesome
  for maximally taking advantage of our architecture and also improving initial
  page load times.

# Completed Steps

- Page Insertion
- Lazy propval init
- Page overview
- Customizable column ordering
- Filter by arbitrary prop
- Sort by arbitrary prop
- Build a column header
- Implement rendering for "empty" propvals
- Authentication
- Ship it
- Add a `created_at` timestamp and expiry. Otherwise, each user only has one
- Get more prop & propval code into traits (i.e, controllers)
  JWT for all time, which is quite cursed
- Implement float
