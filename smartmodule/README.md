# JSON to JSON transformation SmartModule
This is a `map` type SmartModule that transforms JSON records using [Fluvio Jolt](https://github.com/infinyon/fluvio-jolt) library.
The transformation is defined in the configuration of the SmartModule. It is set up once during the SmartModule initialization
and is re-used in the processing.

## Usage
This smartmodule can be used in transforms on a connector like so:
```yaml
transforms:
 - uses: infinyon/jolt@0.3.0
   with:
     spec:
        operation: shift
        spec:
          items:
            "*":
              "@(guid.value)": data[&(1)].guid
              "*":
                "$": data[&(2)].keys[]
```

### Jolt SmartModule transformation example
For the above configuration the `jolt` will process the record:
```json
{
  "description": "top description",
  "items": [
    {
      "description": "inner description 1",
      "guid": {
        "permalink": true,
        "value": "https://example.com/link1-1"
      },
      "link": "https://example.com/link1",
      "pub_date": "Tue, 18 Apr 2023 14:59:04 GMT",
      "title": "Title 1"
    },
    {
      "description": "inner description 2",
      "guid": {
        "permalink": true,
        "value": "https://example.com/link2-1"
      },
      "link": "https://example.com/link2",
      "pub_date": "Tue, 19 Apr 2023 14:20:04 GMT",
      "title": "Title 2"
    }
  ],
  "last_build_date": "Tue, 18 Apr 2023 15:00:01 GMT",
  "link": "https://example.com/top-link",
  "namespaces": {
    "blogChannel": "http://example.com/blogChannelModule"
  },
  "title": "Blog-Recent Entries"
}
```
into:
```json
{
  "data": [
    {
      "guid": "https://example.com/link1-1",
      "keys": [
        "description",
        "guid",
        "link",
        "pub_date",
        "title"
      ]
    },
    {
      "guid": "https://example.com/link2-1",
      "keys": [
        "description",
        "guid",
        "link",
        "pub_date",
        "title"
      ]
    }
  ]
}
```