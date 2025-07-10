(module
    (func (result i32)
      (local i32)

      i32.const 1
      if
        i32.const 1
        i32.const 2
        i32.add
        local.set 0
      else
        i32.const 3
        i32.const 4
        i32.add
        local.set 0
      end

      local.get 0
    )
)
