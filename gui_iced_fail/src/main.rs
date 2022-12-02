use std::{rc::Rc, cell::RefCell};

use iced::{Sandbox, Settings, widget::{text, column, row, button}, Application, Command, Theme, executor, Subscription, subscription, Alignment};
use iced_aw::NumberInput;

fn main() {
    Program::run(Settings::default())
        .unwrap();
}

#[derive(Default)]
struct Program {
    target: isize,
    state: ProgramState,
}

#[derive(Default)]
enum ProgramState {
    #[default]
    GatheringInput,
    AlgorithmExecuting,
    AlgorithmFinished,
}

#[derive(Debug, Clone)]
enum Message {
    StartAlgo,
    TargetChanged(isize),
    AlgoProgress(usize),
}

enum AlgoState {
    Ready(usize),
}

struct AlgoWorker;
fn algo_worker() -> Subscription<Message> {
    subscription::unfold(std::any::TypeId::of::<AlgoWorker>(), AlgoState::Ready(0), |state| async move {
        let n = match state { AlgoState::Ready(n) => n };
        println!("hello from algoworker subscription n={n}");
        (Some(Message::AlgoProgress(0)), AlgoState::Ready(n + 1))
    })
}


impl Application for Program {
    type Message = Message;
    type Theme = Theme;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        (
            Self::default(),
            Command::none()
        )
    }

    fn title(&self) -> String {
        String::from("Adder")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        println!("update called");

        match message {
            Message::StartAlgo => {}
            Message::TargetChanged(new_target) => {
                println!("Target is now {new_target}");
                self.target = new_target;
            }
            Message::AlgoProgress(_) => {}
        }

        Command::none()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        use ProgramState::*;

        match self.state {
            GatheringInput | AlgorithmFinished => Subscription::none(),
            AlgorithmExecuting                 => algo_worker(),
        }
    }

    fn view(&self) -> iced::Element<Self::Message> {
        column![
            text("Adder")
                .width(iced::Length::Shrink)
                .size(50),

            row![
                text("Enter Target in cents")
                    .width(iced::Length::Shrink)
                    .size(25),
                NumberInput::new(self.target, isize::MAX, Message::TargetChanged)
                    .bounds((isize::MIN, isize::MAX)),
            ].spacing(10)
             .padding(10)
             .align_items(Alignment::Center),

            button("Run")
                .width(iced::Length::Shrink)
                .on_press(Self::Message::StartAlgo),
        ]
            .spacing(10)
            .padding(10)
            .align_items(Alignment::Center)
            .into()
    }
}
