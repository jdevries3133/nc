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

# Page Overview

Users will initially load the page in view mode, but we will show an "edit"
button.

On click, we'll swap the rendered page out for a big textarea to edit the
markdown. In this UI, there will be a save button which saves the new content
and switches back to the rendered version.
