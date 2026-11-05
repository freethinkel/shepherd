; One rule per <script>/<style>: the lang attribute picks the language.
((script_element (start_tag) @_tag (raw_text) @injection.content)
  (#not-match? @_tag "lang=[\"']?(ts|typescript)[\"' >]")
  (#set! injection.language "javascript"))
((script_element (start_tag) @_tag (raw_text) @injection.content)
  (#match? @_tag "lang=[\"']?(ts|typescript)[\"' >]")
  (#set! injection.language "typescript"))
((style_element (start_tag) @_tag (raw_text) @injection.content)
  (#not-match? @_tag "lang=[\"']?(scss|less|postcss)[\"' >]")
  (#set! injection.language "css"))
((style_element (start_tag) @_tag (raw_text) @injection.content)
  (#match? @_tag "lang=[\"']?(scss|less|postcss)[\"' >]")
  (#set! injection.language "scss"))
; {expression}, {#if …}, attribute={…}
((svelte_raw_text) @injection.content
  (#set! injection.language "javascript"))
