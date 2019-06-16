# Cibola

_Not_ [ECMA-404](http://www.ecma-international.org/publications/files/ECMA-ST/ECMA-404.pdf) compliant.

Recursive descent "JSON" parser.

## Examples

### Simple

```sh
mkdir examples && \
    curl -O https://raw.githubusercontent.com/zemirco/sf-city-lots-json/master/citylots.json && \
    cargo --release --example simple
```
