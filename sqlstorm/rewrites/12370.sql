
WITH PostStats AS (
    SELECT
        P.Id AS PostId,
        P.Title,
        P.Score,
        P.ViewCount,
        P.AnswerCount,
        P.CommentCount,
        P.CreationDate,
        U.DisplayName AS OwnerDisplayName,
        U.Reputation AS OwnerReputation,
        P.LastEditDate,
        PH.PostHistoryTypeId,
        COUNT(PH.Id) AS HistoryCount
    FROM Posts P
    JOIN Users U ON P.OwnerUserId = U.Id
    LEFT JOIN PostHistory PH ON P.Id = PH.PostId
    GROUP BY P.Id, P.Title, P.Score, P.ViewCount, P.AnswerCount, P.CommentCount, P.CreationDate, U.DisplayName, U.Reputation, P.LastEditDate, PH.PostHistoryTypeId
),
VoteStats AS (
    SELECT
        V.PostId,
        SUM(CASE WHEN V.VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
        SUM(CASE WHEN V.VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes,
        SUM(CASE WHEN V.VoteTypeId = 1 THEN 1 ELSE 0 END) AS AcceptedCount
    FROM Votes V
    GROUP BY V.PostId
)
SELECT
    PS.PostId,
    PS.Title,
    PS.Score,
    PS.ViewCount,
    PS.AnswerCount,
    PS.CommentCount,
    PS.CreationDate,
    PS.OwnerDisplayName,
    PS.OwnerReputation,
    PS.LastEditDate,
    PS.HistoryCount,
    COALESCE(VS.UpVotes, 0) AS UpVotes,
    COALESCE(VS.DownVotes, 0) AS DownVotes,
    COALESCE(VS.AcceptedCount, 0) AS AcceptedCount
FROM PostStats PS
LEFT JOIN VoteStats VS ON PS.PostId = VS.PostId
ORDER BY PS.CreationDate DESC, PS.PostHistoryTypeId
LIMIT 100;
