(module
  (func
    block (result f64) ;; label = @1
      f64.const 0
      f64.const 0
      f64.const 0
      f64.const 0
      f64.const 0
      f64.le
      f64.const 0
      i32.trunc_f64_s
      i32.eqz
      i32.const 32767
      select
      loop (result i64) ;; label = @2
        unreachable
      end
      unreachable
    end
    unreachable
  )
)
