# Notion Clone!

https://github.com/jdevries3133/nc/assets/58614260/ec0d6d17-0e32-429c-96a7-5ef0ca257c26

# Next Steps

1. Implement date
2. Implement datetime
3. Implement multistr (tags)
4. Paginate the collection list view

# Implement Date

Float was a ton more copying than I would have liked but it ultimately didn't
take _that_ long and it was pretty mindless so I think I'm content to continue
the code vomitorium.

# Oopsie -- Should Have Enum'd

I did sort of have a moment of realization that I could do this:

```
enum Value {
  Bool,
  Int,
  Float
}
struct PropVal {
  page_id: i32,
  prop_id: i32,
  value: Value
}

struct Filter {
  // ... other fields
  value: Value
}
```

Then, I can still have nice and type-safe queries. Ultimately, the thing I was
resisting in over-genericizinig my stuff was getting into the territory of
needing to build queries dynamically (dynamically interpolating the table name),
and thus losing compile-time type-safety. With this strategy, though, I can
simply do things like this:

```
// "value_type" presumed to be a variant of `Value` above.
let stuff = match value_type {
  Bool => {
    query_as!(
      "select value from propval_bool where page_id = $1 and prop_id = $2",
      page_id,
      prop_id
    )
  },
  Int => {
    query_as!(
      "select value from propval_int where page_id = $1 and prop_id = $2",
      page_id,
      prop_id
    )
  }
}
```

Clearly, I'll be switching over this enum a whole lot, but I'll also remove a
ton of code.

Nonetheless, I'd prefer to code vomit through all the types, and then do a
mammoth refactor to push the limits of the largest Rust refactoring I've done,
so I can see how that goes.

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
