# JSONA

A jsona parser


```
npm i jsona-js
```

```js
const { parse } = require("jsona-js");

const text = `
{ @openapi
  createUser: { @endpoint({summary: "create a user"})
    route: "POST /users",
    req: {
      body: {
        firstName: "foo",
        lastName: "bar",
      }
    },
    res: {
      200: {
        firstName: "foo",
        lastName: "bar",
        role: "user",
      }
    }
  }
}
`;

const { jsona, error } = parse(text);
```