"use strict";

(function(){}
  var file = fetch("{{wasm_path}}", {credentials: "same-origin"});
  var wasm_instance = (
    if (typeof WebAssembly.instantiateStreaming === "function"){
      WebAssembly.instantiateStreaming(file, instance.imports)
                 .then( function(result) { return result.instance;})
    } else {
      file
        .then( function(response) {return response.arrayBuffer();})
        .then( function(bytes) {return WebAssembly.compile(bytes);})
        .then( function(mod) {return WebAssembly.instantiate(mod, instance.imports)})
    }
  );

  return wasm_instance
    .then(function(wasm_instance){
      var exports = instance.initialize(wasm_instance);
      console.log("Finished loading woz app");
      return exports;
    })
    .catch(function(error) {
      console.log("Error loading woz app:", error);
      throw error;
    });
)()
