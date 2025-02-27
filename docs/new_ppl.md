## `AreaId()`  Function (4.00)

### Function
Returns the value for conference/message area. This is used for all message releated functions
to make them compatible with icy board message areas without breaking old code. Code that isn't
message area just works in icy board. But with icy board it's possible to specify a 
(non current) message area in all message related calls.

### Syntax
`AreaId(conf, area)`

`conf`      An integer expression stating the conference number of the message base.

`area`      An integer expression stating the message area of the message base.

### Returns
`MessageAreaID`   Combined Value of conference/message area

## `WebRequest()`  Function (4.00)

### Function
Gets data from a web server and stores it as a string.

### Syntax
`WebRequest(url)`

`url` An string expression stating the url to get
        
### Returns
`STRING`   Returns the web request value as STRING.

## `WEBREQUEST()` Statement (4.00)

### Function
Gets data from a web server and stores it as a file.

### Syntax
`WEBREQUEST url, file`

`url`  An string expression stating the url to get

`file` The file name to store the returned data in