Notion Clone!

Next Steps

1. (done) Page Insertion
2. (done) Lazy propval init
  - inserted pages will not have any rows in propvals
  - logic for rendering the overview needs to figure out how to deal with that
3. (done) Page overview
  - where page content can be edited
  - markdown time!
4. Customizable column ordering
5. Filter by arbitrary prop
6. Sort by arbitrary prop
7. Deal with treating db_op error as "Not Found"
  - we're treating DB errors as not found in some cases
  - formally, this ain't correct; maybe DB ops should return a 3-member enum of
    `Found<T>`, `NotFound`, or `Error<E>`
8. Implement float
9. Implement date
10. Implement datetime
11. Implement multistr (tags) 
12. Paginate the collection list view

# Customizable Column Ordering

## Summary of Changes

We need some new endpoints, with database operations and components behind them:

```
/collection/:id/prop-order
/prop/:id/order/up
/prop/:id/order/down
```

We need a new column:

```
alter table properties add column order smallint;
```

We need a little icon on the collection table page to link to the column
ordering page.

## Rendering

At `./src/db_ops.rs:231`, we get the set of props for a set of pages. This is
the full set of props represented across all of the pages. Later, pages receive
default values as we loop over them if they do not currently have a stored value
for the relevant prop id.

To implement customizable column ordering technically, this set needs to become
ordered. So, we will change `collection_prop_set` from a hash-set to a vector.
Later, at `./src/db_ops.rs:263`, when we are iterating over the items in this
vector in order, rows of props for each page will correspondingly be generated
in order.

## Persistence

We can persist ordering using a new column on the properties table:

```sql
alter table properties add column order smallint;
```

This will be a nullable field, since explicit ordering will not exist at first.

To set an order for the properties in a collection, each property will need to
be updated.

It does seem like a linked list with self-joins would be more performant for
writes and equally performant for reads (considering the constraints of our
system), but I won't do it for the sake of simplicity.

## Setting Custom Ordering

On the collection view, we will add a sideways hamburger menu in-between where
it says "Create Page," and the first row ("Edit").

I envision a proper header row evolving:

```
Create Page


THIS ICON || title        || age || complete 
Edit         my cool page  |   1  | x |
Edit         other pag...  |   5  | o |
```

The icon is a sideways hamburger because it sort of looks like columns (`|||`).
On hover, it can say ("set column ordering").

On click, we'll jump to a new page, where each of the collections' props are
laid out vertically. Each prop will have a button to bump it up one or down one
in the order :

- name      [up] [down]
- age       [up] [down]
- birthday  [up] [down]

This will send a request to backend endpoints;

```
/prop/:id/order/up
/prop/:id/order/down
```

Each line item in the list will `hx-post` to these endpoints, but the backend
will return a complete new representation of the prop order in response, so the
swap target will always be the whole list.

```
<main>
  <div class="grid grid-cols-3">
    <p>Birthday</p>
    <button hx-post="/prop/1/order/up" hx-target="nearest main">up</button>
    <button hx-post="/prop/1/order/down" hx-target="nearest main">down</button>

    <p>Age</p>
    <button hx-post="/prop/2/order/up" hx-target="nearest main">up</button>
    <button hx-post="/prop/2/order/down" hx-target="nearest main">down</button>
  </div>
</main>
```

Later, for a more close clone of notion, I could use the Sortable JS library,
which HTMX shows for implementing drag-and-drop patterns:
https://htmx.org/examples/sortable/.
