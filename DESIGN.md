# Requirements

## Database View

Mainly, we want to support the rich dynamic database view in notion.

## Pages

We want pages to support arbitrary metadata properties (for the database view).
The body of pages will simply be a markdown document.

## Project Tracking

I want to try to make something that is better than notion at software project
tracking, specifically, in the following ways:

- smart notifications (see "notifications")
- dependency tracking (see "dependency view")
- git integration (see "git integration")

### Notifications

You don't get notified unless you are called out w/ `@` or if you apply a
`following::<your user>` tag to the page.

Later, we can implement rule-sets whereby `following::<your name>` can be
auto-added to pages.

### Dependency View

`page_dependency` will be modeled in the database. Every page can have one or
more dependency. There can be a "show dependencies" dropdown where all of the
direct or indirect dependencies are shown in a tree.

### Git Integration

Let's cut out the middleman and integrate directly with git. Our app will need
access to a git remote URI from which we can fetch updates. Then, every page can
have a "relevant refs" field where commits related to the ticket can be input.
We can also support a tagging syntax like `nc113` in git commit messages, which
can cause that ref to be pulled in as relevant when it's fetched. Then, once we
have this link-up, we can display diffs right in the web UI.

## SQL Swiss-Army Knife

I want to try to make this tool also serve as a frontend for a SQL database,
like Prisma studio. For startups, it would be awesome to have a tool that blends
the lines between notion and SQL.

### Collection = DB Table

A collection is a set of pages. A database table and its set of rows,
therefore, maps onto this abstraction layer.

### Page Props Can Extend Columns

It should be possible to add props to a database collection view. Here, the
notion clone extends the data in the database itself, allowing ad-hoc
experimentation on top of the database schema.

### Page Props can Backfill the Database

After the aformentioned ad-hoc'ing, maybe we want to push props from the notion
clone down into the database. That can be done with a single click!

### Page Content is Internal Notes

Notes about a row in the database will be the content of a page.

### Relations -> Hyperlinks

This is a common pattern for nice DB frontends, but obviously any relation
should work like a link in the frontend. Additionally, if there is page content
("internal notes") on a related row, 

# Roadmap

Things I'm not building yet, but a dumping ground for ideas.

# Tech Notes

## Markdown

We can use [markdown=rs](https://github.com/wooorm/markdown-rs#security), which
is an XSS-safe markdown parser.
