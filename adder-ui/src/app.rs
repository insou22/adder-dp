use std::{rc::Rc, time::Duration};

use adder_ui_model::AlgorithmProgress;
use futures_timer::Delay;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use ybc::*;
use std::boxed::Box;

use crate::future::self_referential_future;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Debug)]
enum NumberSetError {
    InvalidNumber(String)
}

fn number_set(numbers: &str) -> Result<Vec<i64>, NumberSetError> {
    numbers.split_whitespace()
        .filter(|line| !line.is_empty())
        .map(|num| num.parse::<i64>().map_err(|_| num.to_string()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(NumberSetError::InvalidNumber)
}

macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

fn reason_for_invalid_number(offender: &str) -> Html {
    let floating_number_regex = regex!(r"^-?((\d+\.\d*)|(\d*\.\d+))$");
    let number_regex = regex!(r"^-?\d+$");

    if floating_number_regex.is_match(&offender) {
        html! {
            <>
                {"Decimal numbers are not allowed. "}
                {"Work in terms of whole cents, not fractional dollars"}
            </>
        }
    } else if number_regex.is_match(&offender) {
        if offender.starts_with("-") {
            html! {
                <>{ "Number is too low: the program can only handle down to " } { i64::MIN }</>
            }
        } else {
            html! {
                <>{ "Number is too large: the program can only handle up to " } { i64::MAX }</>
            }
        }
    } else {
        html! {
            <>{ "Invalid number: " } { offender }</>
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
struct RunAlgorithmArgs {
    target: i64,
    numberSet: Vec<i64>,
}

enum AppState {
    GatheringInput,
    Calculating(i64, Vec<i64>),
    Result(CalculationComplete),
}

#[function_component(App)]
pub fn app() -> Html {
    let app_state = use_state(|| AppState::GatheringInput);

    let on_calculate = {
        let app_state = app_state.clone();

        Callback::from(move |(target, number_set): (i64, Vec<i64>)| {
            let app_state = app_state.clone();

            log("At callback in app");
            spawn_local(async move {
                log("About to invoke run_algorithm");
                invoke(
                    "run_algorithm",
                    to_value(&RunAlgorithmArgs { target, numberSet: number_set.clone() }).unwrap(),
                )
                .await;
                log("run_algorithm invoked");

                app_state.set(AppState::Calculating(target, number_set));
            });
        })
    };

    let on_complete = {
        let app_state = app_state.clone();

        Callback::from(move |output| {
            log("Completed");
            app_state.set(AppState::Result(output));
        })
    };

    let on_return = {
        let app_state = app_state.clone();

        Callback::from(move |_| {
            app_state.set(AppState::GatheringInput);
        })
    };

    html! {
        <Section>
            <Container fluid={true}>
                <Title>{ "Adder" }</Title>
                {
                    match &*app_state {
                        AppState::GatheringInput => html! {
                            <GatheringInput on_calculate={on_calculate} />
                        },
                        AppState::Calculating(target, number_set) => html! {
                            <Calculating
                                target={*target}
                                number_set={number_set.clone()}
                                on_complete={on_complete}
                            />
                        },
                        AppState::Result(output) => html! {
                            <AlgorithmResult output={output.clone()} on_return={on_return} />
                        },
                    }
                }
            </Container>
        </Section>
    }
}

#[derive(Properties, PartialEq)]
pub struct AlgorithmResultProps {
    output: CalculationComplete,
    on_return: Callback<()>,
}

#[function_component(AlgorithmResult)]
pub fn algorithm_result(props: &AlgorithmResultProps) -> Html {
    let on_return = props.on_return.clone();
    let on_click = Callback::from(move |_| {
        on_return.emit(());
    });

    html! {
        <>
            {
                match props.output.correct_set.as_ref() {
                    Some(correct_set) => {
                        let output = correct_set.iter()
                            .map(|num| num.to_string())
                            .collect::<Vec<_>>()
                            .join("\n");
            
                        html! {
                            <>
                                <Block>
                                    <h3>{ "Found a correct set!" }</h3>
                                </Block>
                                <Block>
                                    <label classes="label">{ "The following numbers add up to exactly " } { props.output.target }</label>
                                    <TextArea name="output" value={output} update={Callback::from(|_| {})} readonly={true} rows={correct_set.len() as u32} />
                                </Block>
                            </>
                        }
                    }
                    None => html! {
                        <>
                            <Block>
                                <h3>{ "Sorry... no exact set exists" }</h3>
                            </Block>
                        </>
                    },
                }
            }
            <Block>
                <Button classes="is-link" onclick={on_click}>{ "Return to menu" }</Button>
            </Block>
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct CalculatingProps {
    target: i64,
    number_set: Vec<i64>,
    on_complete: Callback<CalculationComplete>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalculationComplete {
    target: i64,
    number_set: Vec<i64>,
    correct_set: Option<Vec<i64>>,
}

#[function_component(Calculating)]
pub fn calculating(props: &CalculatingProps) -> Html {
    let progress_out_of = use_state_eq(|| (0, 1));

    let target     = use_ref(|| props.target);
    let number_set = use_ref(|| props.number_set.clone());

    {
        let on_complete = props.on_complete.clone();
        let progress_out_of = progress_out_of.clone();
        let target = target.clone();
        let number_set = number_set.clone();

        spawn_local(self_referential_future(move |future_self| {
            let on_complete = on_complete.clone();
            let progress_out_of = progress_out_of.clone();
            let target = target.clone();
            let number_set = number_set.clone();

            Box::new(async move {
                log("About to invoke check_algorithm");
                let js_value = invoke(
                    "check_algorithm",
                    JsValue::null(),
                )
                .await;
                log("check_algorithm invoked");
        
                let value: AlgorithmProgress = serde_wasm_bindgen::from_value(js_value).unwrap();
        
                match value {
                    AlgorithmProgress::NoAlgorithmRunning => {
                        log("No algorithm running? Ending");
                        return;
                    }
                    AlgorithmProgress::InProgress { progress, out_of } => {
                        log(&format!("In progress: {progress}/{out_of}"));
                        progress_out_of.set((progress, out_of));
                    }
                    AlgorithmProgress::Done(output) => {
                        log(&format!("Done! Output: {output:?}"));
                        on_complete.emit(CalculationComplete {
                            target: *target,
                            number_set: (*number_set).clone(),
                            correct_set: output,
                        });
                        return;
                    }
                }

                Delay::new(Duration::from_millis(100)).await;
        
                spawn_local(future_self.get_self());
            })
        }));
    }

    let progress: f32 = {
        let (progress, out_of) = *progress_out_of;
        (progress as f64 / out_of as f64) as f32
    };

    html! {
        <>
            <Title>{ "Algorithm running..." }</Title>
            <label class="label">{ "Progress: "} { format!("{:.0}", progress * 100.0) } { "%" }</label>
            <Progress classes="is-primary" value={progress} />
        </>
    }
}

#[derive(Properties, PartialEq)]
pub struct GatheringInputProps {
    pub on_calculate: Callback<(i64, Vec<i64>)>,
}

#[function_component(GatheringInput)]
pub fn gathering_input(props: &GatheringInputProps) -> Html {
    let target_input = use_state(|| String::from(""));
    let target_input_callback = {
        let target_input = target_input.clone();
        Callback::from(move |new_value: String| {
            target_input.set(
                new_value.chars()
                    .filter(|&ch| ch.is_ascii_digit() || ch == '-' || ch == '.')
                    .collect()
            );
        })
    };

    let numbers_input = use_state(|| String::from(""));
    let numbers_input_callback = {
        let numbers_input = numbers_input.clone();
        Callback::from(move |new_value: String| {
            numbers_input.set(
                new_value.chars()
                    .filter(|&ch| ch.is_ascii_digit() || ch == '-' || ch == '\r' || ch == '\n' || ch == '.')
                    .collect()
            );
        })
    };

    let target     = Rc::new(target_input.parse::<i64>());
    let number_set = Rc::new(number_set(numbers_input.as_str()));

    let button_clicked = use_state(|| false);

    let button_onclick = {
        let target = target.clone();
        let number_set = number_set.clone();
        let on_calculate = props.on_calculate.clone();
        let button_clicked = button_clicked.clone();

        Callback::from(move |_| {
            button_clicked.set(true);

            let target     = *target.as_ref().as_ref().unwrap();
            let number_set = number_set.as_ref().as_ref().unwrap().clone();

            log("Button clicked, emitting event...");

            on_calculate.emit((target, number_set));
            
            log("...Event emitted.");
        })
    };

    html! {
        <>
            <div class="field">
                <label class="label">{ "Target" }</label>
                <Input
                    name="target"
                    value={target_input.to_string()}
                    update={target_input_callback}
                    placeholder={"Target number"}
                />
            </div>

            <div class="field">
                <label class="label">{ "Numbers (one per line)" }</label>
                <TextArea
                    name="numbers"
                    value={numbers_input.to_string()}
                    update={numbers_input_callback}
                    placeholder="Numbers"
                    rows={10}
                />
            </div>

            {
                if target.is_ok() || (target.is_err() && target_input.as_str() == "") {
                    match &*number_set {
                        Ok(number_set) => {
                            html! {
                                <div class="field">
                                    <Button
                                        classes="is-link"
                                        disabled={*button_clicked || target_input.is_empty() || number_set.is_empty()}
                                        onclick={button_onclick}
                                    >
                                        { "Calculate" }
                                    </Button>
                                </div>
                            }
                        }
                        Err(err) => {
                            match err {
                                NumberSetError::InvalidNumber(offender) => {
                                    html! {
                                        <Notification classes="is-danger">
                                            <p>
                                                { "Invalid number in list: "}
                                                <strong>{ &offender }</strong>
                                            </p>
                                            <p>
                                                { reason_for_invalid_number(&offender) }
                                            </p>
                                        </Notification>
                                    }
                                }
                            }
                        }
                    }
                } else {
                    html! {
                        <Notification classes="is-danger">
                            <p>
                                { "Invalid target number: "}
                                <strong>{ target_input.as_str() }</strong>
                            </p>
                            <p>
                                { reason_for_invalid_number(target_input.as_str()) }
                            </p>
                        </Notification>
                    }

                }
            }
        </>
    }
}


// #[derive(Serialize, Deserialize)]
// struct GreetArgs<'a> {
//     name: &'a str,
// }


// #[function_component(App)]
// pub fn app() -> Html {
//     log("Hello from yew app");

//     let greet_input_ref = use_ref(|| NodeRef::default());

//     let name = use_state(|| String::new());

//     let greet_msg = use_state(|| String::new());
    
//     {
//         let greet_msg = greet_msg.clone();
//         let name = name.clone();
//         let name2 = name.clone();
//         use_effect_with_deps(
//             move |_| {
//                 spawn_local(async move {
//                     if name.is_empty() {
//                         return;
//                     }

//                     // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
//                     let new_msg = invoke(
//                         "greet",
//                         to_value(&GreetArgs { name: &*name }).unwrap(),
//                     )
//                     .await;
//                     log(&new_msg.as_string().unwrap());
//                     greet_msg.set(new_msg.as_string().unwrap());
//                 });

//                 || {}
//             },
//             name2,
//         );
//     }

//     let greet = {
//         let name = name.clone();
//         let greet_input_ref = greet_input_ref.clone();
//         Callback::from(move |_| {
//             name.set(greet_input_ref.cast::<web_sys::HtmlInputElement>().unwrap().value());
//         })
//     };

//     html! {
//         <main class="container">
//             <div class="row">
//                 <a href="https://tauri.app" target="_blank">
//                     <img src="public/tauri.svg" class="logo tauri" alt="Tauri logo"/>
//                 </a>
//                 <a href="https://yew.rs" target="_blank">
//                     <img src="public/yew.png" class="logo yew" alt="Yew logo"/>
//                 </a>
//             </div>

//             <p>{"Click on the Tauri and Yew logos to learn more."}</p>

//             <p>
//                 {"Recommended IDE setup: "}
//                 <a href="https://code.visualstudio.com/" target="_blank">{"VS Code"}</a>
//                 {" + "}
//                 <a href="https://github.com/tauri-apps/tauri-vscode" target="_blank">{"Tauri"}</a>
//                 {" + "}
//                 <a href="https://github.com/rust-lang/rust-analyzer" target="_blank">{"rust-analyzer"}</a>
//             </p>

//             <div class="row">
//                 <input id="greet-input" ref={&*greet_input_ref} placeholder="Enter a name..." />
//                 <button type="button" onclick={greet}>{"Greet"}</button>
//             </div>

//             <p><b>{ &*greet_msg }</b></p>
//         </main>
//     }
// }
