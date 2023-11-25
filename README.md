# Notion Clone!

https://github.com/jdevries3133/nc/assets/58614260/ec0d6d17-0e32-429c-96a7-5ef0ca257c26

# Next Steps

1. Implement datetime
2. Implement string
3. Paginate the collection list view
4. Implement multistr (tags)

# Implement Datetime

Enums should lead the way.

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
- Great propval and filter refactor
