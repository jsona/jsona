# JSONA

A jsona parser


```
npm i jsona
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

parse(text); // jsona ast 
```