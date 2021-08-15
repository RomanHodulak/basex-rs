<PRODUCTS version="1.0">
  {
  for $product in /katalog/polozka
  let $id := data($product/id)
  let $stockItem := /sklad/artikl[kod = $id]
  return <PRODUCT>
    <CODE>{$id}</CODE>
    <STOCK>{data($stockItem/skladem)}</STOCK>
    <DESCRIPTIONS>
      <DESCRIPTION language="cs">
        <TITLE>{data($product/title)}</TITLE>
      </DESCRIPTION>
    </DESCRIPTIONS>
  </PRODUCT>
  }
</PRODUCTS>
