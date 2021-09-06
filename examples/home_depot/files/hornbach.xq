<HORNBACH>
{
  for $i in 1 to 5
  let $catalogue := /Katalog/Produkt[ends-with(Kod, string($i))]
  let $warehouse := /stock/article[id = $i]
  let $packaging := $warehouse/packaging
  return <THING>
    <NAME>{data($catalogue/Nazev[@lang = "en"])}</NAME>
    <WEIGHT>{data($warehouse/weight * 1000)}</WEIGHT>
    <VOLUME>{$packaging/width * $packaging/height * $packaging/depth div 1E6}</VOLUME>
  </THING>
}
</HORNBACH>
