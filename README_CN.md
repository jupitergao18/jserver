# jserver
Rust 编写的 json 接口和静态文件服务器

灵感来自 typicode 采用 nodejs 编写的 json-server

__30秒__ __零代码__ 实现模拟全功能 REST 接口（真的）

为前端开发者倾情打造的快速原型和模拟测试工具。

## 开始使用

安装 JServer 

```
cargo build --release
cp target/release/jserver /usr/bin/
```

创建一个 `data.json` 文件，准备一些数据

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

启动 JServer

```bash
jserver
```

现在你可以直接访问 [http://localhost:2901/api/posts/1](http://localhost:2901/api/posts/1) ，获得数据

```json
{ "id": 1, "title": "jserver", "author": "jupiter.gao" }
```

请求时，你需要知道：

- 当你发送 POST, PUT, PATCH 或 DELETE 请求时，修改的数据将会自动保存到 `data.json` ，并发调用时保存也是安全的。
- 请求体应该是合法的 JSON 对象或单个值。（比如 `{"name": "Foobar"}` `"test string"` `83.01` ）
- 唯一标识（默认为 `id` ）是不可修改的。PUT 或 PATCH 请求中的任何 `id` 值都会被忽略。只有 POST 请求中的 `id` 会使用，不允许重复的 `id` 。
- POST, PUT 或 PATCH 请求头应该指定 `Content-Type: application/json` 。 

## 路由

根据之前的 `data.json` 文件，可以使用以下路由请求接口。 

### 数组 路由

```
GET    /api/posts
GET    /api/posts/1
POST   /api/posts
PUT    /api/posts/1
PATCH  /api/posts/1
DELETE /api/posts/1
```

### 对象或单值 路由

```
GET    /api/profile
POST   /api/profile
PUT    /api/profile
PATCH  /api/profile
```

### 过滤器

```
GET    /api/posts?title=jserver
GET    /api/posts?id=1
```

### 操作符

对于数值，可以使用下列后缀 `_lt`, `_lte`, `_gt`, `_gte` 分别表示 `<`, `<=`, `>`, `>=` 。 
对于字符串，使用 `_like` 表示包含子字符串， `_nlike` 表示不包含子字符串。
对于数组，使用 `_contains` 表示包含元素， `_ncontains` 表示不包含元素。
对于数值、字符串和布尔值，使用 `_ne` 表示 `!=` 。 


```
GET    /api/posts?title_like=server
GET    /api/posts?id_gt=1&id_lt=3
```

### 分页

使用 `_page` 和可选的 `_size` 对返回数据进行分页。

```
GET /api/posts?_page=7
GET /api/posts?_page=7&_size=20
```

_默认每页返回 20 项，页号从 1 开始计数（ 0 当做 1 处理）。_

### 排序

增加 `_sort` 和 `_order` 用来排序。


```
GET /api/posts?_sort=views&_order=asc
```

多字段排序时，按下面格式请求:

```
GET /api/posts?_sort=user,views&_order=desc,asc
```

### 切片

增加 `_start` 和 (`_end` 或 `_limit`)

```
GET /api/posts?_start=20&_end=30
GET /api/posts?_start=20&_limit=10
```

响应头中包含 `X-Total-Count` 用于表示结果总数。

### 库文件

```
GET /db
```

### 上传文件

服务器支持上传文件，并可通过下面介绍的静态文件服务器访问。

```
POST /upload
```

请求体为 `multipart/form-data` 格式，文件字段名为 `file`。
响应体为 json 数组，元素包含 `name`, `path` 和 `size` 字段。

### 静态文件服务器

你可以使用 JServer 提供静态文件服务，如 HTML, JS 和 CSS 文件，只需将文件放在 `./public` 目录即可
或使用 `--public-path` 命令行参数指定其他的静态文件目录。

```bash
mkdir public
echo 'hello world' > public/index.html
jserver
```

```bash
jserver --public-path ./some-other-dir
```

## 命令行参数

```
用法: jserver [选项]

选项:
  -b, --bind-address <服务绑定地址>       [default: 0.0.0.0:2901]
  -d, --db-path <数据json文件>            [default: ./data.json]
  -p, --public-path <静态文件路径>        [default: ./public]
  -i, --id <用作唯一标识的字段名>         [default: id]
  -m, --max-body-limit-m <最大请求限制M>  [default: 100]
      --debug
  -h, --help                              显示帮助信息
  -V, --version                           显示版本号
```

## 许可证

Apache License 2.0
