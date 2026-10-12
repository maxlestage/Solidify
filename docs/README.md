# Solidify's documentation

The pages of the documentation, in [English](en/introduction.md), [French](fr/introduction.md) and [Spanish](es/introduction.md): the same file names in each language. The website shows them at https://maxlestage.github.io/Solidify/docs/, with a menu and a search.

- **Getting started:** `introduction`, `installation`, `quick-start`.
- **The translation:** `how-it-works`, `types`, `ownership`, `classes`, `errors`, `async`, `modules`, `runtime`.
- **Reference:** `cli`, `library`, `mapping`, `limits`, `faq`.

The order and the titles of the pages are in [`site/src/catalog.rs`](../site/src/catalog.rs); the site builds the files in with `include_str!` (`site/src/docs.rs`).

## The examples are checked

`cargo test -p solidify-site` holds every page to what Solidify does:

- a ```` ```ts ```` block must translate;
- a ```` ```ts ```` block followed by a ```` ```rust ```` block must translate to exactly that Rust;
- a ```` ```rust fragment ```` block holds pieces of the translation, separated by `// …` lines, each of which must be in it, in that order;
- a ```` ```ts error ```` block must fail, with the message of the ```` ```text ```` block that follows it;
- a ```` ```ts fragment ```` block (a file of a larger program) is not translated alone;
- a link to a section of a page (`errors.md#…`) must lead to a heading of that page, in the same language.

## Writing a page

- Start with the `#` title of the catalog, then a paragraph: it is the page's description.
- `##` and `###` headings make the page's "On this page"; their anchors are GitHub's, so links work here and on GitHub.
- `> [!NOTE]`, `> [!TIP]` and `> [!WARNING]` are callouts.
- Link to another page by its file name (`cli.md`, `cli.md#options`).
