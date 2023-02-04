declare variable $points external;
<polygon>
  {
    for $i in 1 to $points
    let $angle := 2 * math:pi() * number($i div $points)
    return <point x="{round(math:cos($angle), 8)}" y="{round(math:sin($angle), 8)}"></point>
  }
</polygon>
