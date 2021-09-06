# Home Depot

In this example we have two XML feeds. One is from **warehouse**, second one is from **catalogue**.

Our goal is to combine these feeds together in a specified format. Problem is that the format and units are different.

The result is printed to standard output.

## Input / Output
### Warehouse structure

Feed [`files/warehouse.xml`](files/warehouse.xml)  contains information about the article like dimensions and weight.

| Name                | Unit |
|---------------------|------|
| `weight`            | kg   |
| `packaging/width`   | cm   |
| `packaging/height`  | cm   |
| `packaging/depth`   | cm   |

### Catalogue structure

Feed [`files/catalogue.xml`](files/catalogue.xml)  contains information about the product such as its name and description.

| Name    | Lang   |
|---------|--------|
| `Nazev` | cs, en |

### Output structure

Our custom-made system called **Hornbach** has a special XML import format. It combines the data from above-mentioned feeds into nodes called **THING**. Each **THING** contains these sub-nodes:

#### Textual
| Name   | Lang |
|--------|------|
| `NAME` | en   |

#### Numeric
| Name     | Unit |
|----------|------|
| `WEIGHT` | g    |
| `VOLUME` | m³   |

## Solution

In the source code, we solve this problem in a few steps and get the job done in a single query [`files/hornbach.xq`](files/hornbach.xq).

1. Create an empty database called `hornbach`.
2. Add resource [`files/catalogue.xml`](files/catalogue.xml) to `hornbach`.
3. Add resource [`files/warehouse.xml`](files/warehouse.xml) to `hornbach`.
4. Run query [`files/hornbach.xq`](files/hornbach.xq).
5. Print the result.

### Output
When you run the example, it outputs this XML feed: 

```xml
<HORNBACH>
  <THING>
    <NAME>Shovel</NAME>
    <WEIGHT>526</WEIGHT>
    <VOLUME>4.896</VOLUME>
  </THING>
  <THING>
    <NAME>Pickaxe</NAME>
    <WEIGHT>6799</WEIGHT>
    <VOLUME>3.344</VOLUME>
  </THING>
  <THING>
    <NAME>Gramophone</NAME>
    <WEIGHT>9192</WEIGHT>
    <VOLUME>0.256</VOLUME>
  </THING>
  <THING>
    <NAME>Chest</NAME>
    <WEIGHT>7084</WEIGHT>
    <VOLUME>1.08</VOLUME>
  </THING>
  <THING>
    <NAME>Flowerpot</NAME>
    <WEIGHT>5513</WEIGHT>
    <VOLUME>0.56</VOLUME>
  </THING>
</HORNBACH>
```
