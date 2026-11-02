//! LOGOUT-RL-01 over the same bounded House ingress, without a Channel session.
use super::*;

pub(in crate::gameplay_transport::connection::serve) struct Outcome {
    pub(in crate::gameplay_transport::connection::serve) dispatch: Dispatch,
    pub(in crate::gameplay_transport::connection::serve) second: bool,
    pub(in crate::gameplay_transport::connection::serve) lost: bool,
}

impl Queue {
    pub(in crate::gameplay_transport::connection::serve) async fn logout<S, A>(
        &mut self,
        stream: &mut S,
        authority: &A,
        admitted: AdmittedSession,
    ) -> Result<Outcome, ServeEnd>
    where
        S: AsyncRead + Unpin,
        A: FreshAdmissionAuthority,
    {
        let generation = admitted.continuity.connection_generation;
        let mut second = self
            .held
            .iter()
            .any(|(frame, _)| is_logout_command(frame, generation));
        let mut lost = false;
        let mut logout = std::pin::pin!(authority.logout(admitted));
        let reply = loop {
            let reading = !lost && self.held.len() < MAX_FRAMES_HELD_BEHIND_A_BUFFERED_STEP;
            let mut read = std::pin::pin!(self.frames.next(stream));
            let next = std::future::poll_fn(|cx| {
                if let std::task::Poll::Ready(reply) = logout.as_mut().poll(cx) {
                    return std::task::Poll::Ready(Ok(reply));
                }
                if reading && let std::task::Poll::Ready(frame) = read.as_mut().poll(cx) {
                    return std::task::Poll::Ready(Err(frame));
                }
                std::task::Poll::Pending
            })
            .await;
            match next {
                Ok(reply) => break reply,
                Err(Ok(frame)) => {
                    second |= is_logout_command(&frame, generation);
                    self.held.push_back((frame, false));
                }
                Err(Err(_)) => lost = true,
            }
        };
        let dispatch = match reply {
            LogoutReply::NoLogout => Dispatch::NoLogout,
            LogoutReply::Result(result) => Dispatch::Logout(result),
            LogoutReply::Unknown => return Err(ServeEnd::Disconnected),
        };
        Ok(Outcome {
            dispatch,
            second,
            lost,
        })
    }
}
