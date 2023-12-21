use crate::handle_packet::{Action, HandlingError};


pub async fn handle_action(action: Action){
    match action {
        Action::NoOp(..) =>(), 
        Action::ProcessDatum(..) =>(), 
        Action::ProcessErrorReply(..) =>(), 
        Action::ProcessHelloReply(..) =>(), 
        
        Action::SendHelloReply(..)=>(),
        Action::SendError(..) =>(), 
        Action::SendDatumWithHash(..) =>(), 
        Action::SendPublicKey(..) =>(), 
        Action::SendRoot(..) =>(), 

        Action::StoreRoot(..) =>(), 
        Action::StorePublicKey(..) =>(), 
        _ =>(),
    };
}