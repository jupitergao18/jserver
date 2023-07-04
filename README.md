# jserver
A json api and static files server in rust

Just like json-server from typicode (nodejs)

Get a full fake REST API with __zero coding__ in __less than 30 seconds__ (seriously)

Created with <3 for front-end developers who need a quick back-end for prototyping and mocking.

## Getting started

Install JServer 

```
cargo build --release
cp target/release/jserver /usr/bin/
```

Create a `data.json` file with some data

```json
{
  "posts": [
    { "id": 1, "title": "jserver", "author": "jupiter.gao" }
  ],
  "comments": [
    { "id": 1, "body": "some comment", "postId": 1 }
  ],
  "profile": { "name": "jupiter" },
  "homepage": "https://apicenter.com.cn"
}
```

Start JServer

```bash
jserver
```

Now if you go to [http://localhost:2901/api/posts/1](http://localhost:2901/api/posts/1), you'll get

```json
{ "id": 1, "title": "jserver", "author": "jupiter.gao" }
```

Also when doing requests, it's good to know that:

- If you make POST, PUT, PATCH or DELETE requests, changes will be automatically and safely saved to `data.json`.
- Your request body JSON should be object or single value, just like the GET output. (for example `{"name": "Foobar"}` `"test string"` `83.01`)
- Id values are not mutable. Any `id` value in the body of your PUT or PATCH request will be ignored. Only a value set in a POST request will be respected, but only if not already taken.
- A POST, PUT or PATCH request should include a `Content-Type: application/json` header to use the JSON in the request body. Otherwise it will return a 400 status code. 

## Routes

Based on the previous `data.json` file, here are all the default routes. 

### Array routes

```
GET    /api/posts
GET    /api/posts/1
POST   /api/posts
PUT    /api/posts/1
PATCH  /api/posts/1
DELETE /api/posts/1
```

### Object or Value routes

```
GET    /api/profile
POST   /api/profile
PUT    /api/profile
PATCH  /api/profile
```

### Database

```
GET /db
```

### Static file server

You can use JSON Server to serve your HTML, JS and CSS, simply create a `./public` directory
or use `--public-path` to set a different static files directory.

```bash
mkdir public
echo 'hello world' > public/index.html
jserver
```

```bash
jserver --public-path ./some-other-dir
```

## CLI usage

```
Usage: jserver [OPTIONS]

Options:
  -b, --bind-address <BIND_ADDRESS>  [default: 0.0.0.0:2901]
  -d, --db-path <DB_PATH>            [default: ./data.json]
  -p, --public-path <PUBLIC_PATH>    [default: ./public]
  -i, --id <ID>                      [default: id]
      --debug
  -h, --help                         Print help
  -V, --version                      Print version
```

## License

Apache License 2.0