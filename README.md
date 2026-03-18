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

## To-do

Soon
- Error handling; confirm all panics are safe or justified.
- Add/fix tests.

Later
- Default home page with form to manually submit request.
- Endpoint to list available templates?
- Allow customisation of S3 folder structure (e.g. per date).
- Add tracing logging?
- Cache compilations?
  - Possible to use incremental compilation?
  - Re-use context per template?
- Support document formats other than PDF.
- Handle large files in response.
- OT/Prometheus endpoint?
- Authorisation?
- Rate-limiting?

---

¹I considered using CLI inputs (`--inputs` / `sys.inputs`), but it is surprisingly tedious to convert the JSON string to the expected Typst-internal types. With the virtual file, template development is also simpler --- especially when dealing with large inputs.

