WITH UserBadges AS (
    SELECT UserId, COUNT(*) AS BadgeCount
    FROM Badges
    GROUP BY UserId
), TopUsers AS (
    SELECT U.Id, U.DisplayName, U.Reputation, UB.BadgeCount
    FROM Users U
    LEFT JOIN UserBadges UB ON U.Id = UB.UserId
    WHERE U.Reputation > 1000
), PopularPosts AS (
    SELECT P.Id, P.Title, P.Score, P.ViewCount, P.OwnerUserId, ROW_NUMBER() OVER(ORDER BY P.Score DESC) AS Rank
    FROM Posts P
    WHERE P.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 YEAR'
    AND P.PostTypeId = 1
), PostWithVoteCounts AS (
    SELECT P.Id, P.Title, COALESCE(V.UpVotes, 0) AS UpVotes, COALESCE(V.DownVotes, 0) AS DownVotes
    FROM Posts P
    LEFT JOIN (
        SELECT PostId, SUM(CASE WHEN VoteTypeId = 2 THEN 1 ELSE 0 END) AS UpVotes,
                      SUM(CASE WHEN VoteTypeId = 3 THEN 1 ELSE 0 END) AS DownVotes
        FROM Votes
        GROUP BY PostId
    ) V ON P.Id = V.PostId
), CombinedData AS (
    SELECT TU.DisplayName, TU.Reputation, TU.BadgeCount, PP.Title, PP.Score, PP.ViewCount, PV.UpVotes, PV.DownVotes, PP.Id AS PostId
    FROM TopUsers TU
    JOIN PopularPosts PP ON TU.Id = PP.OwnerUserId
    JOIN PostWithVoteCounts PV ON PP.Id = PV.Id
)
SELECT DisplayName, Reputation, BadgeCount, Title, Score, ViewCount, UpVotes, DownVotes
FROM CombinedData
WHERE BadgeCount >= 5
ORDER BY Reputation DESC, Score DESC, PostId
LIMIT 10;