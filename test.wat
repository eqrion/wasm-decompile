(module
    (func (result i32)
      (local i32)

      i32.const 42
      i32.const 10
      i32.add

      if (result i32)
        i32.const 1
        i32.const 2
        i32.add
      else
        i32.const 3
        i32.const 4
        i32.add
      end
    )
)
