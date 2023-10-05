Notion Clone!

Next Steps

1. Sort by arbitrary prop
2. Implement float
3. Implement date
4. Implement datetime
5. Implement multistr (tags)
6. Paginate the collection list view

# Sort by Arbitrary Prop

Sorting is going to be simple AF compared to filtering.

Users will only be able to sort by one property, so it'll just be a select field
of all the properties.

There will also be a toggle to sort ascending and descending.

Since for now, a collection can only have a single sort, we'll just slap the
`sort_by_prop_id` and `sort_order_type_id` on the `collection` table directly.

We will also add a table `sort_order_type`, which will have two rows: `asc`, and
`desc`.

I have struggled with Postgres and Rust type enums in the scope of this project.
For this sort type, I think I'm going to repeat what I did for `PropValTypes`.
Keep the enum simple, and just do the conversion from int to enum and visa versa
in the database op. I feel like there should be a better way, but I don't know
what it is.

For the UI, we can put a filter button next to our sort button. It will toggle
the display of a toolbar just like the filter toolbar.

The filter toolbar will just contain a select element with each prop, a set of
toggle buttons to switch order between ascending and descending. The select
input will save on change, and the buttons obviously will save on click, where
each action will cause the table to reload.

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

1. (done) Page Insertion
2. (done) Lazy propval init

- inserted pages will not have any rows in propvals
- logic for rendering the overview needs to figure out how to deal with that

3. (done) Page overview

- where page content can be edited
- markdown time!

4. (done) Customizable column ordering
5. (done) Filter by arbitrary prop
