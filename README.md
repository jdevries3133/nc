Notion Clone!

Next Steps

1. Build a column header
2. Implement rendering for "empty" propvals
3. Auth

Release!

2. Implement float
3. Implement date
4. Implement datetime
5. Implement multistr (tags)
6. Paginate the collection list view

# Column Header

There are basically 3 options:

1. mostly maintain the current layout strategy and harangue.
2. CSS grid
3. maintain flex, but use a `flex-col` layout instead

In approach #3, we'd do something like this:

```html
<div class="flex flex-col">

  <!-- This becomes the first column -->
  <div>
    <p>Title One</p>
    <p>Title Two</P
  </div>

  <!-- This becomes the second column, all of the first prop for each page -->
  <div>
    <input type="checkbox" />
    <input type="checkbox" />
  </div>
</div>
```

The big benefit here is that by using flex, we maintain some flexibility in the
layout. It will be easier to add column types as they'll naturally grow /
shrink in width as needed.

However, choosing a pixel width for columns is hardly the most difficult part of
adding new data-types. The downside of flex is it's just more fiddly overall,
and mixed row heights (even trivially) becomes a pain in the butt for alignment.

Thus, I think the best move is to dynamically generate some
`grid-template-columns` CSS for the page. It will end up looking like this:

```css
grid-template-columns: 200px 20px 5px 30px /* ... */
```

So, we're left with a simple place to put the widths of each column and all of
the children will squish to match.

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

1. Page Insertion
2. Lazy propval init

- inserted pages will not have any rows in propvals
- logic for rendering the overview needs to figure out how to deal with that

3. Page overview

- where page content can be edited
- markdown time!

4. Customizable column ordering
5. Filter by arbitrary prop
6. Sort by arbitrary prop
