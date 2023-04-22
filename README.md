# Void

A game my friend and I started developing in 2015, rewritten in Rust to explore wgpu.

"What do we do now?"


## Getting Started

1. Install rust+/cargo via https://rustup.rs
2. Set toolchain to nightly(?)
3. Install the Vulkan SDK (for shaderc) from https://vulkan.lunarg.com/sdk/home
4. Clone and `cargo run`

## [staring into the void (blog post best viewed at dwbrite.com)](https://dwbrite.com/blog/post/staring-into-the-void)

Long ago I went to a 48 hour game jam at Becker University. It was the global game jam and the theme was "what do we do now?" At the time I had already been working on a 2D game project on top of a javafx canvas, so I copied the text rendering code and said "let's make a text adventure!"

It was a fun little project, but little did we know the dangers of such a thing. You see, we were in ye olde java 7 times[1], fresh out of our second year of computer science in high school. We had even struggled to represent branching trees of text that could loop back upon themselves. Perhaps better known as directed graphs.

Our "solution" was to write a [big ol' switch statement](https://github.com/dwbrite/void_2015/blob/6da801eb44577ec7dc2601d8d5f6dc0d827a05d4/src/GameState/Story.java). Teehee 😇 what's a stack overflow?

Text was represented as strings with special tokens to change how fast text was printed. For example, "\\^" meant the following text should print quickly, and "\\#" meant that text should print slow. This has made many people very angry and has been widely regarded as a bad move.

https://user-images.githubusercontent.com/5528368/165872540-a87f7057-652b-415f-9652-08303327cd2d.mp4

A few years later, I tried to run the game out on Linux, and, surprise! 🎉
Audio doesn't play, the logic thread crashes and burns, and you're left with an unresponsive window. Write once, run anywhere, eh?[2]

So 6 years after that, I finally resolved to finish that game the right way. The core idea of void is to have text that is engaging. Sometimes you want text to wiggle, or type-write slowly, or any other of the infinite possibilities to add character to text. Which makes any markup language a natural choice for text representation.

And with that I've started using xml to represent my game text. This required parsing my xml and turning it into data structures stored in bincode files for later use. The last part to navigate then, is branching storylines. I initially decided that this would also be done in xml, so long as the logic isn't much more complicated than checking booleans - but part of me is thinking that maybe these logic checks should be done in Rust.

Anyway, that's all for today! Hopefully I won't get too distracted with other projects in the near future

---

[1] Technically java 8 had just come out the year before, but we were inexperienced - we didn't even know what a lambda was, let alone how to use it. Frankly, I even thought writing code in a legacy style was considered good practice because it was "backwards compatible" 🤦

[2] As long as you're not on linux. And while we're at it, even if you successfully create a cross-platform abstraction layer, you'd need to pack all the abstractions into one distributable. Or create multiple distributables.


