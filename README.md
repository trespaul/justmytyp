# `justmytyp`

An HTTP server for Typst that compiles preconfigured templates with input from HTTP requests.
It replies with a PDF or uploads it to S3.

The `input` field in the request JSON is injected as a virtual file called `input.json` in the compilation context.
In the template it can be accessed with `json("/input.json")`.¹

Run in this repo, and try the following request:

```nu
(http post localhost:3000/
  --content-type application/json
  --full
  {
    name: "foo",
    template: "src/tests/test.typ",
    input: {
      title: "foo",
      text: "bar"
    }
  }
) | get body | save tmp.pdf
```

The compilation context is reused, so the speed should be comparable with `typst watch` (I think).
Hyperfine says 2.3 ms ± 2.8 ms (meaning it's faster than the margin for error in its measurement of the shell startup time),
and Apache's server benchmarking tool completes 1000 requests in under a second (using `ab -n 1000 -c 1000`).

Configuration is done by environment variables.
See the `Config` struct and its `Default` implementation for the variables and their defaults.

## To-do

### Soon

In addition to the in-text notes, the following are priorities.

- Error handling; confirm all panics are safe or justified.
- Avoid unnecessary allocations.
- Add/fix tests.
- Investigate startup time lag.
- NixOS module.

### Later

These are ideas for improvements.

- Allow customisation of S3 folder structure (e.g. per date).
- Support document formats other than PDF.
- Refresh fonts (font library is currently only built once at startup).
- Add tracing logging?
- Endpoint to list available templates?
- Handle large files in response and multipart s3 upload.
- OT/Prometheus endpoint?
- Authorisation?
- Rate-limiting?


## Credit

The implementation of the compiler context and the interaction with the Typst libraries was based on [tfachmann's example code](https://github.com/tfachmann/typst-as-library/blob/main/src/lib.rs).

---

¹I considered using CLI inputs (`--inputs` / `sys.inputs`), but it is surprisingly tedious to convert the JSON string to the expected Typst-internal types. With the virtual file, template development is also simpler --- especially when dealing with large inputs.

