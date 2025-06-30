(module
  (type (;0;) (func (result i32)))
  (import "imports" "memory" (memory (;0;) 1))
  (export "getPublicSuffixPos" (func 0))
  (func (;0;) (type 0) (result i32)
    (local i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32 i32)
    i32.const 404
    i32.load
    local.set 0
    i32.const 400
    i32.load
    i32.const 2
    i32.shl
    local.set 1
    i32.const 256
    local.set 2
    i32.const -1
    local.set 3
    block ;; label = @1
      loop ;; label = @2
        local.get 2
        i32.load8_u
        local.get 2
        i32.load8_u offset=1
        local.tee 4
        i32.sub
        local.set 5
        local.get 1
        i32.load16_u offset=2
        local.tee 10
        i32.eqz
        br_if 1 (;@1;)
        local.get 1
        i32.load offset=8
        i32.const 2
        i32.shl
        local.set 7
        i32.const 0
        local.tee 9
        local.set 8
        block ;; label = @3
          loop ;; label = @4
            local.get 9
            local.get 10
            i32.ge_u
            br_if 1 (;@3;)
            local.get 9
            local.get 10
            i32.add
            i32.const 1
            i32.shr_u
            local.tee 12
            i32.const 2
            i32.shl
            local.tee 16
            local.get 16
            i32.const 1
            i32.shl
            i32.add
            local.get 7
            i32.add
            local.tee 13
            i32.load8_u
            local.set 14
            local.get 5
            local.get 14
            i32.sub
            local.tee 11
            i32.eqz
            if ;; label = @5
              local.get 14
              i32.const 4
              i32.le_u
              if ;; label = @6
                local.get 13
                i32.const 4
                i32.add
                local.set 15
              else
                local.get 0
                local.get 13
                i32.load offset=4
                i32.add
                local.set 15
              end
              local.get 4
              local.tee 16
              local.get 5
              i32.add
              local.set 18
              local.get 15
              local.set 17
              block ;; label = @6
                loop ;; label = @7
                  local.get 16
                  i32.load8_u
                  local.get 17
                  i32.load8_u
                  i32.sub
                  local.tee 11
                  br_if 1 (;@6;)
                  local.get 16
                  i32.const 1
                  i32.add
                  local.tee 16
                  local.get 18
                  i32.eq
                  br_if 1 (;@6;)
                  local.get 17
                  i32.const 1
                  i32.add
                  local.set 17
                  br 0 (;@7;)
                end
              end
            end
            local.get 11
            i32.const 0
            i32.lt_s
            if ;; label = @5
              local.get 12
              local.set 10
              br 1 (;@4;)
            end
            local.get 11
            i32.const 0
            i32.gt_s
            if ;; label = @5
              local.get 12
              i32.const 1
              i32.add
              local.set 9
              br 1 (;@4;)
            end
            local.get 13
            local.set 8
          end
        end
        local.get 8
        i32.eqz
        if ;; label = @3
          local.get 7
          i32.load offset=4
          i32.const 42
          i32.ne
          br_if 2 (;@1;)
          i32.const 399
          i32.const 1
          i32.store8
          local.get 7
          local.set 8
        end
        local.get 8
        local.tee 1
        i32.load8_u offset=1
        local.tee 16
        i32.const 2
        i32.and
        if ;; label = @3
          local.get 2
          i32.const 256
          i32.gt_u
          if ;; label = @4
            local.get 2
            i32.const -2
            i32.add
            return
          end
          br 2 (;@1;)
        end
        local.get 16
        i32.const 1
        i32.and
        if ;; label = @3
          local.get 2
          local.set 3
        end
        local.get 4
        i32.eqz
        br_if 1 (;@1;)
        local.get 2
        i32.const 2
        i32.add
        local.set 2
        br 0 (;@2;)
      end
    end
    local.get 3
  )
)
