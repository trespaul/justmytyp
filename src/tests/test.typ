#import "@preview/polylux:0.4.0": *

#set page(paper: "presentation-16-9")


#set text(
  font: "Lato",
  size: 23pt,
)

#let data = json("/input.json")

= #data.title

#data.text

$ y = m x + n $
