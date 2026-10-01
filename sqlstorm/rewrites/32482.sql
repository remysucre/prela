WITH RecentPosts AS (
 SELECT
 p.Id,
 p.Title,
 p.CreationDate,
 p.Score,
 p.ViewCount,
 p.AnswerCount,
 p.OwnerUserId,
 ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.CreationDate DESC, p.Id DESC) as rn
 FROM
 Posts p
 WHERE
 p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year'
),
UserScores AS (
 SELECT
 u.Id AS UserId,
 u.DisplayName,
 SUM(CASE WHEN voteType.VoteTypeId = 2 THEN 1 ELSE 0 END) - SUM(CASE WHEN voteType.VoteTypeId = 3 THEN 1 ELSE 0 END) AS ReputationScore,
 COUNT(DISTINCT bh.Id) AS BadgeCount
 FROM
 Users u
 LEFT JOIN
 Votes voteType ON u.Id = voteType.UserId
 LEFT JOIN
 Badges bh ON u.Id = bh.UserId
 GROUP BY
 u.Id, u.DisplayName
),
TopUsers AS (
 SELECT
 UserId,
 DisplayName,
 ReputationScore,
 BadgeCount,
 ROW_NUMBER() OVER (ORDER BY ReputationScore DESC, UserId) AS UserRank
 FROM
 UserScores
),
PostHistoryAnalysis AS (
 SELECT
 ph.PostId,
 ph.UserId,
 COUNT(*) AS EditCount,
 MAX(ph.CreationDate) AS LastEditDate,
 STRING_AGG(DISTINCT pht.Name, ', ' ORDER BY pht.Name) AS ChangeTypes
 FROM
 PostHistory ph
 JOIN
 PostHistoryTypes pht ON ph.PostHistoryTypeId = pht.Id
 GROUP BY
 ph.PostId, ph.UserId
)
SELECT
 rp.Title AS RecentPostTitle,
 rp.CreationDate AS PostCreationDate,
 rp.Score AS PostScore,
 rp.ViewCount AS PostViewCount,
 rp.AnswerCount AS NumberOfAnswers,
 tu.DisplayName AS TopUserDisplayName,
 tu.ReputationScore AS UserReputationScore,
 ph.LastEditDate AS LastEditDate,
 ph.EditCount AS EditCount,
 ph.ChangeTypes AS TypesOfChanges
FROM
 RecentPosts rp
LEFT JOIN
 TopUsers tu ON rp.OwnerUserId = tu.UserId
LEFT JOIN
 PostHistoryAnalysis ph ON rp.Id = ph.PostId
WHERE
 rp.rn = 1
 AND tu.UserRank <= 10
ORDER BY
 rp.CreationDate DESC;
