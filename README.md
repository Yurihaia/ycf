# YCF
a config format i wrote for no real reason.

```
key = { // this is a map.
    list = [ // this is a list.
        "this is a string"
        -100000 // this is an integer
        null // null
        1.5 // a float
    ]
    // nothing ever needs commas
    
    // this:
    other_map.key1.key2 = "hello"
    // is a shorthand for this.
    equivalent_to = { key1 = { key2 = "hello" } }
}
// thats it. thats the whole format.
```