struct Struct{

}

// split list of tokens by ast node: `;`, `}`, `]`` (] *ONLY* for #root, #default, and #derive)

// the graph is not a tree; it is a simple stack of ASTNodes (then instructions during compilation)
// if statements