[中文版说明](README_CN.md)

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
    { "id": 1, "body": "some comment" }
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

### Filter

```
GET    /api/posts?title=jserver
GET    /api/posts?id=1
```

### Operators

For numbers, use the following suffix: `_lt`, `_lte`, `_gt`, `_gte` for `<`, `<=`, `>`, `>=` respectively. 
For strings, use `_like` for `contains` and `_nlike` for `not contains`. 
For arrays, use `_contains` for `contains` and `_ncontains` for `not contains`. 
For numbers, strings, booleans, use `_ne` for `!=`. 


```
GET    /api/posts?title_like=server
GET    /api/posts?id_gt=1&id_lt=3
```

### Paginate

Use `_page` and optionally `_size` to paginate returned data.

```
GET /api/posts?_page=7
GET /api/posts?_page=7&_size=20
```

_20 items are returned by default, page is 1 based(0 is treated as 1)_

### Sort

Add `_sort` and `_order` (ascending order by default)

```
GET /api/posts?_sort=views&_order=asc
```

For multiple fields, use the following format:

```
GET /api/posts?_sort=user,views&_order=desc,asc
```

### Slice

Add `_start` and (`_end` or `_limit`)

```
GET /api/posts?_start=20&_end=30
GET /api/posts?_start=20&_limit=10
```

An `X-Total-Count` header is included in the array response

### Database

```
GET /db
```

### Upload files

You can upload files to the server and access them through the static file server below.

```
POST /upload
```

Request body should be `multipart/form-data` and file field name should be `file`.
Response body will be a JSON array with each item having `name`, `path` and `size` properties.

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
  -b, --bind-address <BIND_ADDRESS>          [default: 0.0.0.0:2901]
  -d, --db-path <DB_PATH>                    [default: ./data.json]
  -p, --public-path <PUBLIC_PATH>            [default: ./public]
  -i, --id <ID>                              [default: id]
  -m, --max-body-limit-m <MAX_BODY_LIMIT_M>  [default: 100]
      --debug
  -h, --help                                 Print help
  -V, --version                              Print version
```

## License

Apache License 2.0
