
WITH UserReputation AS (
    SELECT
        U.Id AS UserId,
        U.DisplayName,
        U.Reputation,
        ROW_NUMBER() OVER (ORDER BY U.Reputation DESC) AS Rank
    FROM Users U
),
ActiveQuestions AS (
    SELECT
        P.Id AS QuestionId,
        P.Title,
        P.CreationDate,
        COALESCE(P.AcceptedAnswerId, 0) AS AcceptedAnswerId,
        COUNT(CASE WHEN C.Id IS NOT NULL THEN 1 END) AS CommentCount,
        AVG(CASE WHEN V.VoteTypeId = 2 THEN 1.0 ELSE 0 END) AS AvgUpVotes
    FROM Posts P
    LEFT JOIN Comments C ON P.Id = C.PostId
    LEFT JOIN Votes V ON P.Id = V.PostId AND V.VoteTypeId = 2
    WHERE P.PostTypeId = 1 
    GROUP BY P.Id, P.Title, P.AcceptedAnswerId, P.CreationDate
),
ClosedQuestions AS (
    SELECT
        PH.PostId,
        COUNT(*) AS CloseCount
    FROM PostHistory PH
    WHERE PH.PostHistoryTypeId = 10
    GROUP BY PH.PostId
),
FinalResults AS (
    SELECT
        AQ.QuestionId,
        AQ.Title,
        AQ.AcceptedAnswerId,
        AQ.CommentCount,
        AQ.AvgUpVotes,
        COALESCE(CQ.CloseCount, 0) AS CloseCount,
        UR.Reputation
    FROM ActiveQuestions AQ
    LEFT JOIN ClosedQuestions CQ ON AQ.QuestionId = CQ.PostId
    JOIN UserReputation UR ON AQ.AcceptedAnswerId = UR.UserId
)

SELECT 
    FR.QuestionId,
    FR.Title,
    FR.CommentCount,
    FR.AvgUpVotes,
    FR.CloseCount,
    CASE 
        WHEN FR.Reputation > 1000 THEN 'High Reputation'
        WHEN FR.Reputation BETWEEN 500 AND 1000 THEN 'Medium Reputation'
        ELSE 'Low Reputation'
    END AS ReputationCategory
FROM FinalResults FR
WHERE FR.CloseCount = 0
ORDER BY FR.AvgUpVotes DESC, FR.CommentCount DESC, FR.QuestionId
FETCH FIRST 100 ROWS ONLY;
