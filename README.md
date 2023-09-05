Notion Clone!

Next Steps

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

Handling lazy propval

- we probably want to gather the set of prop ids in the page being displayed
- after looking up propvals, we can actually create propvals that don't exist
  "on the fly" since we know all the necessary values!
  - prop id is the id of the prop we're missing a value for
  - page id is the id of the page!
  - value can be a default, depending on the propval type
- at that point, we've initted a valid `models::PvInt` (for example); we can
  simply call the `save` method if we want to persist it.
    - the current `save` impl uses SQL `update`.
    - We should change it to upsert instead of update!
