import { useHashParam } from "@/lib/utils";
import type { FormComponent } from "./Form";
import type { FormModel } from "./models";
import { createFormReducer } from "./reducer";
import { useCallback, useEffect, useReducer, useState } from "react";
import { useRouter } from "next/navigation";
import { invoke } from "@tauri-apps/api/tauri";
import { Form } from "./Form";
import { FormView } from "./FormView";
import { Header, BackAction, DeleteAction, SaveAction } from "./Header";

export interface Configuration<T extends FormModel> {
  initData: T;
  crud: {
    deleteId: string;
    saveId: string;
    getId: string;
    createId: string;
  };
  path: string;
}

export function FormWrapper<T extends FormModel>(config: Configuration<T>) {
  const Wrapper = (Component: FormComponent<T>) => {
    const _Wrapper = () => {
      const hashParam = useHashParam();

      const [state, dispatch] = useReducer(
        createFormReducer<T>(),
        config.initData,
      );
      const router = useRouter();

      const performGetItem = useCallback(
        async (_id: number) => {
          const result = await invoke<T>(config.crud.getId, { id: _id });
          if (result) {
            dispatch({ type: "put", payload: result });
          }
        },
        [dispatch],
      );

      const performAfterCreated = async (item?: T) => {
        if (item) {
          router.replace(`${config.path}#id=${item.id}`);
          dispatch({ type: "put", payload: item });
        }
      };

      const performAfterUpdated = async (item?: T) => {
        if (item) await performGetItem(item.id);
      };

      const performSave = async () => {
        const id = state.id;
        if (id === 0) {
          const { id, ...data } = state;
          const result = await invoke<T>(config.crud.createId, { data });
          await performAfterCreated(result);
        } else if (id > 0) {
          const result = await invoke<T>(config.crud.saveId, { data: state });
          await performAfterUpdated(result);
        }
      };

      const performDelete = useCallback(async () => {
        if (state.id > 0) {
          const id = state.id;
          const result = await invoke<boolean>(config.crud.deleteId, { id });
          if (result) router.back();
        }
      }, [state.id]);

      useEffect(() => {
        const id = hashParam.getNumber("id");
        if (id > 0) {
          dispatch({ type: "set/id", payload: id });
        }
      }, [hashParam]);

      useEffect(() => {
        if (state.id > 0) {
          performGetItem(state.id);
        }
      }, [state.id, performGetItem]);

      return (
        <FormView>
          <Header
            actions={
              <>
                <BackAction />
                <SaveAction handle={performSave} />
                {state.id ? <DeleteAction handle={performDelete} /> : ""}
              </>
            }
          />
          <Form>
            <Component state={state} dispatch={dispatch} />
          </Form>
        </FormView>
      );
    };

    return _Wrapper;
  };

  return Wrapper;
}
