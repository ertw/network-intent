module Browser

import NetDSL.Compiler

%default total

-- The JavaScript backend accepts a browser-specific foreign implementation.
-- This is the only browser boundary; compiler semantics remain Idris code.
%foreign "browser:lambda:(evaluate)=>()=>{globalThis.netcEvaluate=(source)=>evaluate(source);}"
registerEvaluator : (String -> String) -> IO ()

main : IO ()
main = registerEvaluator (evaluateSource "browser.net")
