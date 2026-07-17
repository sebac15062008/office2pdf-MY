# Bug-fix evidence

Store visual regression evidence in an issue-numbered directory:

```text
assets/bugfixes/issue-<number>/gt.jpg
assets/bugfixes/issue-<number>/before.jpg
assets/bugfixes/issue-<number>/after.jpg
```

Generate all images from the same input document, page, resolution, and renderer. Use progressive JPEG quality 86 with metadata stripped.

Tracking issues may store discovery evidence under `audit/<case>/` with `gt.jpg` and `before.jpg`; focused child fixes add their own `after.jpg`.
